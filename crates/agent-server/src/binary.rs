//! Binary message protocol: 4-byte big-endian header length + JSON header + raw payload.

use serde::{Deserialize, Serialize};

/// Header for binary upload chunks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryChunkHeader {
    pub id: String,
    pub upload_id: String,
    pub file_path: String,
    pub offset: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub checksum: String,
}

/// Header for binary artwork images.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryArtworkHeader {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub app_id: u32,
    pub artwork_type: String,
    pub content_type: String,
}

/// Parsed binary message — either a chunk or an artwork image.
#[derive(Debug)]
pub enum BinaryMessage {
    Chunk {
        header: BinaryChunkHeader,
        data: Vec<u8>,
    },
    Artwork {
        header: BinaryArtworkHeader,
        data: Vec<u8>,
    },
}

/// Parses a raw binary WebSocket frame into a [`BinaryMessage`].
///
/// Wire format: `[4 bytes: header_len (big-endian)][header_len bytes: JSON][rest: payload]`
pub fn parse_binary_message(data: &[u8]) -> Result<BinaryMessage, ParseError> {
    if data.len() < 4 {
        return Err(ParseError::TooShort);
    }

    let header_len = (data[0] as usize) << 24
        | (data[1] as usize) << 16
        | (data[2] as usize) << 8
        | (data[3] as usize);

    if data.len() < 4 + header_len {
        return Err(ParseError::HeaderTruncated {
            expected: header_len,
            got: data.len() - 4,
        });
    }

    let header_bytes = &data[4..4 + header_len];
    let payload = data[4 + header_len..].to_vec();

    // Peek at the JSON to determine type.
    let peek: HeaderPeek =
        serde_json::from_slice(header_bytes).map_err(|e| ParseError::InvalidJson(e.to_string()))?;

    if peek.msg_type.as_deref() == Some("artwork_image") {
        let header: BinaryArtworkHeader = serde_json::from_slice(header_bytes)
            .map_err(|e| ParseError::InvalidJson(e.to_string()))?;
        Ok(BinaryMessage::Artwork {
            header,
            data: payload,
        })
    } else {
        let header: BinaryChunkHeader = serde_json::from_slice(header_bytes)
            .map_err(|e| ParseError::InvalidJson(e.to_string()))?;
        Ok(BinaryMessage::Chunk {
            header,
            data: payload,
        })
    }
}

/// Encodes a binary message for sending over WebSocket.
#[allow(dead_code)] // Will be used by handler implementations in future crates.
pub fn encode_binary_message<T: Serialize>(
    header: &T,
    payload: &[u8],
) -> Result<Vec<u8>, serde_json::Error> {
    let header_json = serde_json::to_vec(header)?;
    let header_len = header_json.len() as u32;

    let mut buf = Vec::with_capacity(4 + header_json.len() + payload.len());
    buf.extend_from_slice(&header_len.to_be_bytes());
    buf.extend_from_slice(&header_json);
    buf.extend_from_slice(payload);
    Ok(buf)
}

#[derive(Deserialize)]
struct HeaderPeek {
    #[serde(rename = "type")]
    msg_type: Option<String>,
}

/// Errors from binary message parsing.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("message too short (need at least 4 bytes)")]
    TooShort,

    #[error("header truncated: expected {expected} bytes, got {got}")]
    HeaderTruncated { expected: usize, got: usize },

    #[error("invalid header JSON: {0}")]
    InvalidJson(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_binary_frame(header: &[u8], payload: &[u8]) -> Vec<u8> {
        let len = header.len() as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(&len.to_be_bytes());
        buf.extend_from_slice(header);
        buf.extend_from_slice(payload);
        buf
    }

    #[test]
    fn parse_chunk_message() {
        let header = serde_json::to_vec(&serde_json::json!({
            "id": "msg-1",
            "uploadId": "u-1",
            "filePath": "game.exe",
            "offset": 0,
            "checksum": "abc123"
        }))
        .unwrap();
        let payload = b"binary data here";

        let frame = make_binary_frame(&header, payload);
        let msg = parse_binary_message(&frame).unwrap();

        match msg {
            BinaryMessage::Chunk { header, data } => {
                assert_eq!(header.id, "msg-1");
                assert_eq!(header.upload_id, "u-1");
                assert_eq!(header.file_path, "game.exe");
                assert_eq!(header.offset, 0);
                assert_eq!(header.checksum, "abc123");
                assert_eq!(data, payload);
            }
            _ => panic!("expected Chunk variant"),
        }
    }

    #[test]
    fn parse_artwork_message() {
        let header = serde_json::to_vec(&serde_json::json!({
            "id": "msg-2",
            "type": "artwork_image",
            "appId": 12345,
            "artworkType": "grid",
            "contentType": "image/png"
        }))
        .unwrap();
        let payload = b"\x89PNG\r\n\x1a\n";

        let frame = make_binary_frame(&header, payload);
        let msg = parse_binary_message(&frame).unwrap();

        match msg {
            BinaryMessage::Artwork { header, data } => {
                assert_eq!(header.id, "msg-2");
                assert_eq!(header.msg_type, "artwork_image");
                assert_eq!(header.app_id, 12345);
                assert_eq!(header.artwork_type, "grid");
                assert_eq!(header.content_type, "image/png");
                assert_eq!(data, payload);
            }
            _ => panic!("expected Artwork variant"),
        }
    }

    #[test]
    fn parse_too_short() {
        let result = parse_binary_message(&[0, 0, 0]);
        assert!(matches!(result, Err(ParseError::TooShort)));
    }

    #[test]
    fn parse_header_truncated() {
        // Header says 100 bytes but only has 5.
        let data = [0, 0, 0, 100, 1, 2, 3, 4, 5];
        let result = parse_binary_message(&data);
        assert!(matches!(result, Err(ParseError::HeaderTruncated { .. })));
    }

    #[test]
    fn parse_invalid_json() {
        let frame = make_binary_frame(b"not json", b"payload");
        let result = parse_binary_message(&frame);
        assert!(matches!(result, Err(ParseError::InvalidJson(_))));
    }

    #[test]
    fn encode_roundtrip() {
        let header = BinaryChunkHeader {
            id: "r-1".into(),
            upload_id: "u-1".into(),
            file_path: "test.bin".into(),
            offset: 512,
            checksum: String::new(),
        };
        let payload = b"roundtrip data";

        let encoded = encode_binary_message(&header, payload).unwrap();
        let parsed = parse_binary_message(&encoded).unwrap();

        match parsed {
            BinaryMessage::Chunk { header: h, data } => {
                assert_eq!(h.id, "r-1");
                assert_eq!(h.upload_id, "u-1");
                assert_eq!(h.offset, 512);
                assert_eq!(data, payload);
            }
            _ => panic!("expected Chunk"),
        }
    }

    #[test]
    fn empty_payload() {
        let header = serde_json::to_vec(&serde_json::json!({
            "id": "msg-3",
            "uploadId": "u-2",
            "filePath": "empty.bin",
            "offset": 0
        }))
        .unwrap();

        let frame = make_binary_frame(&header, &[]);
        let msg = parse_binary_message(&frame).unwrap();

        match msg {
            BinaryMessage::Chunk { data, .. } => assert!(data.is_empty()),
            _ => panic!("expected Chunk"),
        }
    }
}
