use serde::{Deserialize, Serialize};

/// Sent when console log streaming is enabled/disabled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleLogStatusEvent {
    pub enabled: bool,
    pub level_mask: u32,
}

/// A text segment with optional CSS styling from console `%c` directives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyledSegment {
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub css: String,
}

/// A single console log entry from CEF/CDP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsoleLogEntry {
    pub timestamp: i64,
    pub level: String,
    pub source: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub line: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<StyledSegment>,
}

/// A batch of console log entries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsoleLogBatch {
    pub entries: Vec<ConsoleLogEntry>,
    pub dropped: i32,
}

fn is_zero_i32(v: &i32) -> bool {
    *v == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_log_entry_roundtrip() {
        let entry = ConsoleLogEntry {
            timestamp: 1700000000,
            level: "error".into(),
            source: "console".into(),
            text: "Something went wrong".into(),
            url: "https://example.com/app.js".into(),
            line: 42,
            segments: vec![
                StyledSegment {
                    text: "Error: ".into(),
                    css: "color: red".into(),
                },
                StyledSegment {
                    text: "details".into(),
                    css: String::new(),
                },
            ],
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: ConsoleLogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, parsed);
    }

    #[test]
    fn console_log_entry_omit_empty() {
        let entry = ConsoleLogEntry {
            timestamp: 1700000000,
            level: "log".into(),
            source: "console".into(),
            text: "hello".into(),
            url: String::new(),
            line: 0,
            segments: vec![],
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(!json.contains("url"));
        assert!(!json.contains("line"));
        assert!(!json.contains("segments"));
    }

    #[test]
    fn console_log_batch_roundtrip() {
        let batch = ConsoleLogBatch {
            entries: vec![ConsoleLogEntry {
                timestamp: 1,
                level: "warn".into(),
                source: "js".into(),
                text: "warning".into(),
                url: String::new(),
                line: 0,
                segments: vec![],
            }],
            dropped: 5,
        };
        let json = serde_json::to_string(&batch).unwrap();
        let parsed: ConsoleLogBatch = serde_json::from_str(&json).unwrap();
        assert_eq!(batch, parsed);
    }

    #[test]
    fn styled_segment_omit_empty_css() {
        let seg = StyledSegment {
            text: "plain".into(),
            css: String::new(),
        };
        let json = serde_json::to_string(&seg).unwrap();
        assert!(!json.contains("css"));
    }

    #[test]
    fn console_log_status_roundtrip() {
        let status = ConsoleLogStatusEvent {
            enabled: true,
            level_mask: 15,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"levelMask\":15"));
        let parsed: ConsoleLogStatusEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(status, parsed);
    }
}
