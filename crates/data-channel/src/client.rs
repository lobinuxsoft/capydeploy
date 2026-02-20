//! TCP data client (hub side).
//!
//! Connects to the agent's ephemeral TCP listener, authenticates with
//! the token, and streams files.

use std::net::SocketAddr;
use std::path::PathBuf;

use tokio::io::{AsyncReadExt, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use crate::error::DataChannelError;
use crate::wire::{
    FileHeader, read_auth_response, write_end_marker, write_file_header, write_token,
};
use crate::{TCP_AUTH_TIMEOUT, TCP_BUFFER_SIZE, TCP_CONNECT_TIMEOUT};

/// TCP data client for sending file transfers.
pub struct TcpDataClient;

impl TcpDataClient {
    /// Connects to the agent, authenticates, and streams all files.
    ///
    /// `files` is a list of `(local_path, relative_path)` pairs.
    /// Returns the total bytes sent.
    pub async fn connect_and_send(
        addr: SocketAddr,
        token: &str,
        files: &[(PathBuf, String)],
        cancel: CancellationToken,
        progress_tx: mpsc::Sender<(i64, String)>,
    ) -> Result<i64, DataChannelError> {
        // Connect with timeout + cancellation.
        let stream = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                return Err(DataChannelError::Cancelled);
            }
            result = tokio::time::timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect(addr)) => {
                match result {
                    Ok(Ok(s)) => {
                        info!(%addr, "TCP data channel connected");
                        s
                    }
                    Ok(Err(e)) => return Err(e.into()),
                    Err(_) => return Err(DataChannelError::Timeout),
                }
            }
        };

        let (reader, writer) = stream.into_split();
        let mut reader = reader;
        let mut writer = BufWriter::with_capacity(TCP_BUFFER_SIZE, writer);

        // Send token.
        write_token(&mut writer, token).await?;
        tokio::io::AsyncWriteExt::flush(&mut writer).await?;

        // Wait for auth response with timeout.
        let accepted = tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                return Err(DataChannelError::Cancelled);
            }
            result = tokio::time::timeout(TCP_AUTH_TIMEOUT, read_auth_response(&mut reader)) => {
                match result {
                    Ok(Ok(a)) => a,
                    Ok(Err(e)) => return Err(e),
                    Err(_) => return Err(DataChannelError::Timeout),
                }
            }
        };

        if !accepted {
            return Err(DataChannelError::AuthFailed("agent rejected token".into()));
        }

        info!("TCP data channel: authenticated");

        // Stream files.
        let mut total_bytes: i64 = 0;
        let mut buf = vec![0u8; TCP_BUFFER_SIZE];

        for (local_path, relative_path) in files {
            if cancel.is_cancelled() {
                return Err(DataChannelError::Cancelled);
            }

            let metadata = tokio::fs::metadata(local_path).await?;
            let file_size = metadata.len();

            let header = FileHeader {
                relative_path: relative_path.clone(),
                file_size,
            };
            write_file_header(&mut writer, &header).await?;

            let mut file = tokio::fs::File::open(local_path).await?;
            let mut remaining = file_size;

            while remaining > 0 {
                if cancel.is_cancelled() {
                    return Err(DataChannelError::Cancelled);
                }

                let to_read = (remaining as usize).min(buf.len());
                let n = file.read(&mut buf[..to_read]).await?;
                if n == 0 {
                    return Err(DataChannelError::Protocol(
                        "unexpected EOF reading local file".into(),
                    ));
                }

                tokio::io::AsyncWriteExt::write_all(&mut writer, &buf[..n]).await?;
                remaining -= n as u64;
                total_bytes += n as i64;

                // Report progress (non-blocking).
                let _ = progress_tx.try_send((total_bytes, relative_path.clone()));
            }

            debug!(
                path = %relative_path,
                size = file_size,
                "TCP data channel: file sent"
            );
        }

        // Send end marker and flush.
        write_end_marker(&mut writer).await?;
        tokio::io::AsyncWriteExt::flush(&mut writer).await?;

        info!(total_bytes, "TCP data channel: all files sent");
        Ok(total_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::TcpDataServer;

    /// Integration test: client -> server full pipeline with temp files.
    #[tokio::test]
    async fn client_server_integration() {
        let server_dir = tempfile::tempdir().unwrap();
        let client_dir = tempfile::tempdir().unwrap();

        // Create test files on the client side.
        let file1_path = client_dir.path().join("game.exe");
        let file2_path = client_dir.path().join("data.bin");
        std::fs::write(&file1_path, b"EXECUTABLE_DATA").unwrap();
        std::fs::write(&file2_path, b"BINARY_DATA_HERE").unwrap();

        let files = vec![
            (file1_path.clone(), "game.exe".to_string()),
            (file2_path.clone(), "data.bin".to_string()),
        ];

        let cancel = CancellationToken::new();
        let server = TcpDataServer::new(server_dir.path().to_path_buf(), cancel.clone());
        let (info, listener) = server.listen().await.unwrap();

        let addr: SocketAddr = format!("127.0.0.1:{}", info.port).parse().unwrap();
        let token = info.token.clone();

        let (server_progress_tx, _server_progress_rx) = mpsc::channel(64);
        let (client_progress_tx, _client_progress_rx) = mpsc::channel(64);

        // Spawn server.
        let server_handle = tokio::spawn(async move {
            server
                .accept_and_receive(listener, &token, server_progress_tx)
                .await
        });

        // Run client.
        let client_result =
            TcpDataClient::connect_and_send(addr, &info.token, &files, cancel, client_progress_tx)
                .await;

        let client_bytes = client_result.unwrap();
        let server_bytes = server_handle.await.unwrap().unwrap();

        assert_eq!(client_bytes, server_bytes);

        // Verify files on disk.
        let received_exe = std::fs::read(server_dir.path().join("game.exe")).unwrap();
        assert_eq!(received_exe, b"EXECUTABLE_DATA");

        let received_data = std::fs::read(server_dir.path().join("data.bin")).unwrap();
        assert_eq!(received_data, b"BINARY_DATA_HERE");
    }

    /// Test subdirectory creation during transfer.
    #[tokio::test]
    async fn client_server_subdirectories() {
        let server_dir = tempfile::tempdir().unwrap();
        let client_dir = tempfile::tempdir().unwrap();

        // Create a nested file.
        std::fs::create_dir_all(client_dir.path().join("assets/textures")).unwrap();
        let file_path = client_dir.path().join("assets/textures/hero.png");
        std::fs::write(&file_path, b"PNG_DATA").unwrap();

        let files = vec![(file_path, "assets/textures/hero.png".to_string())];

        let cancel = CancellationToken::new();
        let server = TcpDataServer::new(server_dir.path().to_path_buf(), cancel.clone());
        let (info, listener) = server.listen().await.unwrap();

        let addr: SocketAddr = format!("127.0.0.1:{}", info.port).parse().unwrap();
        let token = info.token.clone();
        let (s_tx, _) = mpsc::channel(64);
        let (c_tx, _) = mpsc::channel(64);

        let server_handle =
            tokio::spawn(async move { server.accept_and_receive(listener, &token, s_tx).await });

        TcpDataClient::connect_and_send(addr, &info.token, &files, cancel, c_tx)
            .await
            .unwrap();

        server_handle.await.unwrap().unwrap();

        let received = std::fs::read(server_dir.path().join("assets/textures/hero.png")).unwrap();
        assert_eq!(received, b"PNG_DATA");
    }

    /// Test cancellation mid-transfer.
    #[tokio::test]
    async fn client_cancellation() {
        let server_dir = tempfile::tempdir().unwrap();
        let client_dir = tempfile::tempdir().unwrap();

        // Create a large-ish file.
        let data = vec![0xABu8; 1_000_000];
        let file_path = client_dir.path().join("big.bin");
        std::fs::write(&file_path, &data).unwrap();

        let files = vec![(file_path, "big.bin".to_string())];

        let cancel = CancellationToken::new();
        let server = TcpDataServer::new(server_dir.path().to_path_buf(), cancel.clone());
        let (info, listener) = server.listen().await.unwrap();

        let addr: SocketAddr = format!("127.0.0.1:{}", info.port).parse().unwrap();
        let token = info.token.clone();
        let (s_tx, _) = mpsc::channel(64);
        let (c_tx, _) = mpsc::channel(64);

        // Cancel before client starts.
        cancel.cancel();

        let server_handle =
            tokio::spawn(async move { server.accept_and_receive(listener, &token, s_tx).await });

        let result = TcpDataClient::connect_and_send(addr, &info.token, &files, cancel, c_tx).await;

        assert!(matches!(result, Err(DataChannelError::Cancelled)));

        // Server should also see cancellation or error.
        let server_result = server_handle.await.unwrap();
        assert!(server_result.is_err());
    }

    /// Test progress reporting.
    #[tokio::test]
    async fn progress_reporting() {
        let server_dir = tempfile::tempdir().unwrap();
        let client_dir = tempfile::tempdir().unwrap();

        let data = vec![0xCDu8; TCP_BUFFER_SIZE * 3];
        let file_path = client_dir.path().join("multi_chunk.bin");
        std::fs::write(&file_path, &data).unwrap();

        let files = vec![(file_path, "multi_chunk.bin".to_string())];

        let cancel = CancellationToken::new();
        let server = TcpDataServer::new(server_dir.path().to_path_buf(), cancel.clone());
        let (info, listener) = server.listen().await.unwrap();

        let addr: SocketAddr = format!("127.0.0.1:{}", info.port).parse().unwrap();
        let token = info.token.clone();
        let (s_tx, _s_rx) = mpsc::channel(64);
        let (c_tx, mut c_rx) = mpsc::channel(64);

        let server_handle =
            tokio::spawn(async move { server.accept_and_receive(listener, &token, s_tx).await });

        TcpDataClient::connect_and_send(addr, &info.token, &files, cancel, c_tx)
            .await
            .unwrap();

        server_handle.await.unwrap().unwrap();

        // Collect client progress events.
        let mut client_events = Vec::new();
        while let Ok(evt) = c_rx.try_recv() {
            client_events.push(evt);
        }
        assert!(!client_events.is_empty(), "should have progress events");

        // Last event should report all bytes.
        let last = client_events.last().unwrap();
        assert_eq!(last.0, data.len() as i64);
    }

    /// Test empty file transfer.
    #[tokio::test]
    async fn empty_file_transfer() {
        let server_dir = tempfile::tempdir().unwrap();
        let client_dir = tempfile::tempdir().unwrap();

        let file_path = client_dir.path().join("empty.txt");
        std::fs::write(&file_path, b"").unwrap();

        let files = vec![(file_path, "empty.txt".to_string())];

        let cancel = CancellationToken::new();
        let server = TcpDataServer::new(server_dir.path().to_path_buf(), cancel.clone());
        let (info, listener) = server.listen().await.unwrap();

        let addr: SocketAddr = format!("127.0.0.1:{}", info.port).parse().unwrap();
        let token = info.token.clone();
        let (s_tx, _) = mpsc::channel(64);
        let (c_tx, _) = mpsc::channel(64);

        let server_handle =
            tokio::spawn(async move { server.accept_and_receive(listener, &token, s_tx).await });

        let result = TcpDataClient::connect_and_send(addr, &info.token, &files, cancel, c_tx).await;

        assert_eq!(result.unwrap(), 0);
        server_handle.await.unwrap().unwrap();

        let received = std::fs::read(server_dir.path().join("empty.txt")).unwrap();
        assert!(received.is_empty());
    }
}
