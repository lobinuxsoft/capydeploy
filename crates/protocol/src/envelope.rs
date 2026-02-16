use serde::{Deserialize, Serialize};

use crate::constants::MessageType;

/// Error details in a WebSocket message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WsError {
    pub code: i32,
    pub message: String,
}

/// Envelope for all WebSocket communication.
///
/// The `payload` field uses `serde_json::value::RawValue` to defer deserialization,
/// matching Go's `json.RawMessage` behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Box<serde_json::value::RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<WsError>,
}

impl Message {
    /// Creates a new message with the given type and payload.
    pub fn new<T: Serialize>(
        id: impl Into<String>,
        msg_type: MessageType,
        payload: Option<&T>,
    ) -> Result<Self, serde_json::Error> {
        let raw = match payload {
            Some(p) => {
                let json = serde_json::to_string(p)?;
                Some(serde_json::value::RawValue::from_string(json)?)
            }
            None => None,
        };
        Ok(Self {
            id: id.into(),
            msg_type,
            payload: raw,
            error: None,
        })
    }

    /// Deserializes the payload into the given type.
    pub fn parse_payload<T: for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<Option<T>, serde_json::Error> {
        match &self.payload {
            Some(raw) => Ok(Some(serde_json::from_str(raw.get())?)),
            None => Ok(None),
        }
    }

    /// Creates an error message.
    pub fn error(id: impl Into<String>, code: i32, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            msg_type: MessageType::Error,
            payload: None,
            error: Some(WsError {
                code,
                message: message.into(),
            }),
        }
    }

    /// Creates a response message for this request.
    pub fn reply<T: Serialize>(
        &self,
        msg_type: MessageType,
        payload: Option<&T>,
    ) -> Result<Self, serde_json::Error> {
        Message::new(&self.id, msg_type, payload)
    }

    /// Creates an error response for this request.
    pub fn reply_error(&self, code: i32, message: impl Into<String>) -> Self {
        Message::error(&self.id, code, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::InfoResponse;
    use crate::types::AgentInfo;

    #[test]
    fn message_new_with_payload() {
        let payload = serde_json::json!({"key": "value"});
        let msg = Message::new("msg-1", MessageType::GetInfo, Some(&payload)).unwrap();
        assert_eq!(msg.id, "msg-1");
        assert_eq!(msg.msg_type, MessageType::GetInfo);
        assert!(msg.payload.is_some());
        assert!(msg.error.is_none());
    }

    #[test]
    fn message_new_without_payload() {
        let msg = Message::new::<()>("msg-2", MessageType::Ping, None).unwrap();
        assert!(msg.payload.is_none());
    }

    #[test]
    fn message_error_creation() {
        let msg = Message::error("msg-3", 400, "bad request");
        assert_eq!(msg.msg_type, MessageType::Error);
        let err = msg.error.unwrap();
        assert_eq!(err.code, 400);
        assert_eq!(err.message, "bad request");
    }

    #[test]
    fn message_parse_payload() {
        let info = AgentInfo {
            id: "a1".into(),
            name: "Agent".into(),
            platform: "linux".into(),
            version: "0.1.0".into(),
            accept_connections: true,
            supported_image_formats: vec![],
        };
        let resp = InfoResponse {
            agent: info.clone(),
        };
        let msg = Message::new("m1", MessageType::InfoResponse, Some(&resp)).unwrap();
        let parsed: Option<InfoResponse> = msg.parse_payload().unwrap();
        assert_eq!(parsed.unwrap().agent, info);
    }

    #[test]
    fn message_json_roundtrip() {
        let msg = Message::error("e1", 500, "internal");
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "e1");
        assert_eq!(parsed.msg_type, MessageType::Error);
        assert!(parsed.error.is_some());
        assert!(parsed.payload.is_none());
    }

    #[test]
    fn message_omits_null_fields() {
        let msg = Message::new::<()>("m1", MessageType::Ping, None).unwrap();
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("payload"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn reply_preserves_id() {
        let original = Message::new::<()>("req-42", MessageType::Ping, None).unwrap();
        let reply = original
            .reply(MessageType::Pong, Some(&serde_json::json!({})))
            .unwrap();
        assert_eq!(reply.id, "req-42");
        assert_eq!(reply.msg_type, MessageType::Pong);
    }

    #[test]
    fn reply_error_preserves_id() {
        let original = Message::new::<()>("req-99", MessageType::GetInfo, None).unwrap();
        let reply = original.reply_error(404, "not found");
        assert_eq!(reply.id, "req-99");
        assert_eq!(reply.msg_type, MessageType::Error);
    }
}
