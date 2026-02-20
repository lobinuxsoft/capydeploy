//! TCP wire format for file transfers.
//!
//! # Wire format
//!
//! ```text
//! HANDSHAKE (Hub -> Agent):     [32 bytes: hex token ASCII]
//! AUTH RESPONSE (Agent -> Hub): [1 byte: 0x01=OK, 0x00=rejected]
//!
//! PER FILE (Hub -> Agent):
//!   [2 bytes BE: path_len]
//!   [path_len bytes: relative_path UTF-8]
//!   [8 bytes BE: file_size]
//!   [file_size bytes: raw file data]
//!
//! END MARKER: [2 bytes: 0x0000]
//! ```

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::DataChannelError;

/// Authentication response: accepted.
pub const AUTH_OK: u8 = 0x01;

/// Authentication response: rejected.
pub const AUTH_REJECTED: u8 = 0x00;

/// Token length in bytes (32 hex characters).
pub const TOKEN_LEN: usize = 32;

/// A file header in the TCP stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHeader {
    /// Relative path (UTF-8, forward slashes).
    pub relative_path: String,
    /// Total file size in bytes.
    pub file_size: u64,
}

/// Writes a file header to the stream.
pub async fn write_file_header<W: AsyncWrite + Unpin>(
    writer: &mut W,
    header: &FileHeader,
) -> Result<(), DataChannelError> {
    let path_bytes = header.relative_path.as_bytes();
    let path_len = path_bytes.len();
    if path_len > u16::MAX as usize {
        return Err(DataChannelError::Protocol(format!(
            "path too long: {path_len} bytes (max {})",
            u16::MAX
        )));
    }

    writer.write_u16(path_len as u16).await?;
    writer.write_all(path_bytes).await?;
    writer.write_u64(header.file_size).await?;
    Ok(())
}

/// Reads a file header from the stream.
///
/// Returns `None` if the end marker (path_len == 0) is encountered.
pub async fn read_file_header<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Option<FileHeader>, DataChannelError> {
    let path_len = reader.read_u16().await?;
    if path_len == 0 {
        return Ok(None);
    }

    let mut path_buf = vec![0u8; path_len as usize];
    reader.read_exact(&mut path_buf).await?;
    let relative_path = String::from_utf8(path_buf)
        .map_err(|e| DataChannelError::Protocol(format!("invalid UTF-8 path: {e}")))?;

    let file_size = reader.read_u64().await?;

    Ok(Some(FileHeader {
        relative_path,
        file_size,
    }))
}

/// Writes the end-of-transfer marker (path_len = 0).
pub async fn write_end_marker<W: AsyncWrite + Unpin>(
    writer: &mut W,
) -> Result<(), DataChannelError> {
    writer.write_u16(0).await?;
    Ok(())
}

/// Writes the authentication token (32 hex ASCII bytes).
pub async fn write_token<W: AsyncWrite + Unpin>(
    writer: &mut W,
    token: &str,
) -> Result<(), DataChannelError> {
    if token.len() != TOKEN_LEN {
        return Err(DataChannelError::Protocol(format!(
            "token must be {TOKEN_LEN} bytes, got {}",
            token.len()
        )));
    }
    writer.write_all(token.as_bytes()).await?;
    Ok(())
}

/// Reads the authentication token (32 hex ASCII bytes).
pub async fn read_token<R: AsyncRead + Unpin>(reader: &mut R) -> Result<String, DataChannelError> {
    let mut buf = [0u8; TOKEN_LEN];
    reader.read_exact(&mut buf).await?;
    String::from_utf8(buf.to_vec())
        .map_err(|e| DataChannelError::Protocol(format!("invalid token encoding: {e}")))
}

/// Writes the authentication response byte.
pub async fn write_auth_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
    accepted: bool,
) -> Result<(), DataChannelError> {
    writer
        .write_u8(if accepted { AUTH_OK } else { AUTH_REJECTED })
        .await?;
    writer.flush().await?;
    Ok(())
}

/// Reads the authentication response byte.
pub async fn read_auth_response<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<bool, DataChannelError> {
    let byte = reader.read_u8().await?;
    Ok(byte == AUTH_OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn file_header_roundtrip() {
        let header = FileHeader {
            relative_path: "game/data/level1.bin".into(),
            file_size: 1_048_576,
        };

        let mut buf = Vec::new();
        write_file_header(&mut buf, &header).await.unwrap();

        let mut cursor = &buf[..];
        let parsed = read_file_header(&mut cursor).await.unwrap().unwrap();
        assert_eq!(parsed, header);
    }

    #[tokio::test]
    async fn end_marker_detection() {
        let mut buf = Vec::new();
        write_end_marker(&mut buf).await.unwrap();

        let mut cursor = &buf[..];
        let result = read_file_header(&mut cursor).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn token_roundtrip() {
        let token = "a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8";

        let mut buf = Vec::new();
        write_token(&mut buf, token).await.unwrap();

        let mut cursor = &buf[..];
        let parsed = read_token(&mut cursor).await.unwrap();
        assert_eq!(parsed, token);
    }

    #[tokio::test]
    async fn auth_response_accepted() {
        let mut buf = Vec::new();
        write_auth_response(&mut buf, true).await.unwrap();
        assert_eq!(buf[0], AUTH_OK);

        let mut cursor = &buf[..];
        assert!(read_auth_response(&mut cursor).await.unwrap());
    }

    #[tokio::test]
    async fn auth_response_rejected() {
        let mut buf = Vec::new();
        write_auth_response(&mut buf, false).await.unwrap();
        assert_eq!(buf[0], AUTH_REJECTED);

        let mut cursor = &buf[..];
        assert!(!read_auth_response(&mut cursor).await.unwrap());
    }

    #[tokio::test]
    async fn multiple_files_then_end_marker() {
        let files = vec![
            FileHeader {
                relative_path: "file1.txt".into(),
                file_size: 100,
            },
            FileHeader {
                relative_path: "dir/file2.bin".into(),
                file_size: 200,
            },
        ];

        let mut buf = Vec::new();
        for f in &files {
            write_file_header(&mut buf, f).await.unwrap();
        }
        write_end_marker(&mut buf).await.unwrap();

        let mut cursor = &buf[..];
        let h1 = read_file_header(&mut cursor).await.unwrap().unwrap();
        assert_eq!(h1, files[0]);

        let h2 = read_file_header(&mut cursor).await.unwrap().unwrap();
        assert_eq!(h2, files[1]);

        let end = read_file_header(&mut cursor).await.unwrap();
        assert!(end.is_none());
    }

    #[tokio::test]
    async fn invalid_token_length() {
        let mut buf = Vec::new();
        let result = write_token(&mut buf, "too_short").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn path_too_long() {
        let long_path = "a".repeat(u16::MAX as usize + 1);
        let header = FileHeader {
            relative_path: long_path,
            file_size: 0,
        };
        let mut buf = Vec::new();
        let result = write_file_header(&mut buf, &header).await;
        assert!(result.is_err());
    }
}
