//! TCP data server (agent side).
//!
//! Binds an ephemeral port, accepts a single connection, validates the
//! token, and receives files streamed by the Hub.

use std::path::{Path, PathBuf};

use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::error::DataChannelError;
use crate::wire::{read_file_header, read_token, write_auth_response};
use crate::{TCP_AUTH_TIMEOUT, TCP_BUFFER_SIZE, TCP_CONNECT_TIMEOUT};

/// Info returned after binding the listener (sent to Hub via WS).
#[derive(Debug, Clone)]
pub struct DataChannelInfo {
    pub port: u16,
    pub token: String,
}

/// TCP data server for receiving file transfers.
pub struct TcpDataServer {
    base_path: PathBuf,
    cancel: CancellationToken,
}

impl TcpDataServer {
    pub fn new(base_path: PathBuf, cancel: CancellationToken) -> Self {
        Self { base_path, cancel }
    }

    /// Binds an ephemeral TCP listener and generates a token.
    ///
    /// Returns the channel info (port + token) and the listener.
    /// The caller should send the info to the Hub via WS, then call
    /// [`accept_and_receive`](Self::accept_and_receive).
    pub async fn listen(&self) -> Result<(DataChannelInfo, TcpListener), DataChannelError> {
        let listener = TcpListener::bind("0.0.0.0:0").await?;
        let port = listener.local_addr()?.port();
        let token = crate::token::generate_token();

        info!(port, "TCP data channel listener bound");

        Ok((DataChannelInfo { port, token }, listener))
    }

    /// Accepts a single connection, validates the token, and receives files.
    ///
    /// Returns the total bytes received across all files.
    pub async fn accept_and_receive(
        &self,
        listener: TcpListener,
        expected_token: &str,
        progress_tx: mpsc::Sender<(i64, String)>,
    ) -> Result<i64, DataChannelError> {
        // Wait for connection with timeout + cancellation.
        let stream = tokio::select! {
            biased;
            _ = self.cancel.cancelled() => {
                return Err(DataChannelError::Cancelled);
            }
            result = tokio::time::timeout(TCP_CONNECT_TIMEOUT, listener.accept()) => {
                match result {
                    Ok(Ok((stream, addr))) => {
                        info!(%addr, "TCP data channel connection accepted");
                        stream
                    }
                    Ok(Err(e)) => return Err(e.into()),
                    Err(_) => return Err(DataChannelError::Timeout),
                }
            }
        };

        // Drop listener — only one connection per session.
        drop(listener);

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::with_capacity(TCP_BUFFER_SIZE, reader);

        // Authenticate with timeout.
        let received_token = tokio::select! {
            biased;
            _ = self.cancel.cancelled() => {
                return Err(DataChannelError::Cancelled);
            }
            result = tokio::time::timeout(TCP_AUTH_TIMEOUT, read_token(&mut reader)) => {
                match result {
                    Ok(Ok(t)) => t,
                    Ok(Err(e)) => return Err(e),
                    Err(_) => return Err(DataChannelError::Timeout),
                }
            }
        };

        if !crate::token::validate_token(&received_token, expected_token) {
            warn!("TCP data channel: invalid token");
            write_auth_response(&mut writer, false).await?;
            return Err(DataChannelError::AuthFailed("invalid token".into()));
        }

        write_auth_response(&mut writer, true).await?;
        info!("TCP data channel: authenticated");

        // Receive files.
        let mut total_bytes: i64 = 0;
        let mut buf = vec![0u8; TCP_BUFFER_SIZE];

        loop {
            if self.cancel.is_cancelled() {
                return Err(DataChannelError::Cancelled);
            }

            let header = match read_file_header(&mut reader).await? {
                Some(h) => h,
                None => {
                    debug!("TCP data channel: end marker received");
                    break;
                }
            };

            // Validate path (same rules as transfer crate).
            validate_upload_path(&header.relative_path)?;

            let file_path = self.base_path.join(&header.relative_path);
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            let mut file = tokio::fs::File::create(&file_path).await?;
            let mut remaining = header.file_size;

            while remaining > 0 {
                if self.cancel.is_cancelled() {
                    return Err(DataChannelError::Cancelled);
                }

                let to_read = (remaining as usize).min(buf.len());
                let n = reader.read(&mut buf[..to_read]).await?;
                if n == 0 {
                    return Err(DataChannelError::Protocol(
                        "unexpected EOF during file data".into(),
                    ));
                }

                tokio::io::AsyncWriteExt::write_all(&mut file, &buf[..n]).await?;
                remaining -= n as u64;
                total_bytes += n as i64;

                // Report progress (non-blocking).
                let _ = progress_tx.try_send((total_bytes, header.relative_path.clone()));
            }

            debug!(
                path = %header.relative_path,
                size = header.file_size,
                "TCP data channel: file received"
            );
        }

        info!(total_bytes, "TCP data channel: transfer complete");
        Ok(total_bytes)
    }
}

/// Validates a relative file path for safety (no traversal, no absolute paths).
fn validate_upload_path(path: &str) -> Result<(), DataChannelError> {
    if path.is_empty() {
        return Err(DataChannelError::InvalidPath("empty path".into()));
    }

    if Path::new(path).is_absolute() {
        return Err(DataChannelError::InvalidPath(format!(
            "absolute path not allowed: {path}"
        )));
    }

    // Reject parent traversal.
    for component in Path::new(path).components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(DataChannelError::InvalidPath(format!(
                "parent traversal not allowed: {path}"
            )));
        }
    }

    // Reject Windows-style prefixes.
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return Err(DataChannelError::InvalidPath(format!(
            "Windows drive prefix not allowed: {path}"
        )));
    }
    if path.starts_with("\\\\") {
        return Err(DataChannelError::InvalidPath(format!(
            "UNC path not allowed: {path}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_path_rejects_empty() {
        assert!(validate_upload_path("").is_err());
    }

    #[test]
    fn validate_path_rejects_absolute() {
        assert!(validate_upload_path("/etc/passwd").is_err());
    }

    #[test]
    fn validate_path_rejects_parent_traversal() {
        assert!(validate_upload_path("../secret").is_err());
        assert!(validate_upload_path("a/../../etc").is_err());
    }

    #[test]
    fn validate_path_rejects_windows_drive() {
        assert!(validate_upload_path("C:\\Windows").is_err());
    }

    #[test]
    fn validate_path_rejects_unc() {
        assert!(validate_upload_path("\\\\server\\share").is_err());
    }

    #[test]
    fn validate_path_allows_normal() {
        assert!(validate_upload_path("game.exe").is_ok());
        assert!(validate_upload_path("data/level1.bin").is_ok());
        assert!(validate_upload_path("./game.exe").is_ok());
    }

    #[tokio::test]
    async fn server_reject_bad_token() {
        let dir = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let server = TcpDataServer::new(dir.path().to_path_buf(), cancel.clone());
        let (info, listener) = server.listen().await.unwrap();

        let (progress_tx, _progress_rx) = mpsc::channel(16);

        // Connect with wrong token.
        let addr = format!("127.0.0.1:{}", info.port);
        let server_task = tokio::spawn(async move {
            server
                .accept_and_receive(listener, &info.token, progress_tx)
                .await
        });

        let mut stream = tokio::net::TcpStream::connect(&addr).await.unwrap();
        // Send wrong token.
        let bad_token = "00000000000000000000000000000000";
        tokio::io::AsyncWriteExt::write_all(&mut stream, bad_token.as_bytes())
            .await
            .unwrap();

        // Read auth response.
        let mut resp = [0u8; 1];
        tokio::io::AsyncReadExt::read_exact(&mut stream, &mut resp)
            .await
            .unwrap();
        assert_eq!(resp[0], crate::wire::AUTH_REJECTED);

        let result = server_task.await.unwrap();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn server_cancellation() {
        let dir = tempfile::tempdir().unwrap();
        let cancel = CancellationToken::new();
        let server = TcpDataServer::new(dir.path().to_path_buf(), cancel.clone());
        let (info, listener) = server.listen().await.unwrap();
        let _ = info; // suppress unused warning

        let (progress_tx, _progress_rx) = mpsc::channel(16);

        // Cancel before anyone connects.
        cancel.cancel();

        let result = server
            .accept_and_receive(listener, "dummy_token_00000000000000000000", progress_tx)
            .await;
        assert!(matches!(result, Err(DataChannelError::Cancelled)));
    }
}
