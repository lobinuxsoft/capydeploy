fn main() {
    println!("Run `cargo test -p wire-compat` to execute wire compatibility tests.");
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    /// Returns the path to the fixtures directory.
    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
    }

    /// Loads a fixture JSON file and returns it as a `serde_json::Value`.
    fn load_fixture(name: &str) -> serde_json::Value {
        let path = fixtures_dir().join(name);
        let data = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
        serde_json::from_str(&data)
            .unwrap_or_else(|e| panic!("failed to parse fixture {}: {e}", path.display()))
    }

    /// Normalizes JSON values so that integer-valued floats compare equal.
    ///
    /// Go serializes `float64(65)` as `65`, Rust serializes `f64` as `65.0`.
    /// Both are semantically identical. This function normalizes numbers so
    /// that `65` and `65.0` compare as equal.
    fn normalize_value(v: &serde_json::Value) -> serde_json::Value {
        match v {
            serde_json::Value::Number(n) => {
                // If it's representable as f64, use f64 (normalizes int vs float)
                if let Some(f) = n.as_f64() {
                    serde_json::json!(f)
                } else {
                    v.clone()
                }
            }
            serde_json::Value::Object(map) => {
                let normalized: serde_json::Map<String, serde_json::Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), normalize_value(v)))
                    .collect();
                serde_json::Value::Object(normalized)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(normalize_value).collect())
            }
            _ => v.clone(),
        }
    }

    /// Deserializes a fixture into a Rust type, re-serializes it, and compares
    /// the JSON values (order-independent, float-normalized comparison).
    fn roundtrip_test<T>(name: &str)
    where
        T: serde::de::DeserializeOwned + serde::Serialize,
    {
        let fixture = load_fixture(name);
        let parsed: T = serde_json::from_value(fixture.clone())
            .unwrap_or_else(|e| panic!("failed to deserialize {name}: {e}"));
        let reserialized = serde_json::to_value(&parsed)
            .unwrap_or_else(|e| panic!("failed to re-serialize {name}: {e}"));

        let norm_fixture = normalize_value(&fixture);
        let norm_reserialized = normalize_value(&reserialized);
        assert_eq!(
            norm_fixture, norm_reserialized,
            "roundtrip mismatch for {name}:\n  Go:   {fixture}\n  Rust: {reserialized}"
        );
    }

    // --- Protocol type tests ---

    #[test]
    fn fixture_message_envelope() {
        if !fixtures_dir().join("message_envelope.json").exists() {
            eprintln!("SKIP: fixture not generated yet (run Go TestGenerateFixtures)");
            return;
        }
        roundtrip_test::<capydeploy_protocol::Message>("message_envelope.json");
    }

    #[test]
    fn fixture_agent_info() {
        if !fixtures_dir().join("agent_info.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::AgentInfo>("agent_info.json");
    }

    #[test]
    fn fixture_hub_connected_request() {
        if !fixtures_dir().join("hub_connected_request.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::messages::HubConnectedRequest>(
            "hub_connected_request.json",
        );
    }

    #[test]
    fn fixture_agent_status_response() {
        if !fixtures_dir().join("agent_status_response.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::messages::AgentStatusResponse>(
            "agent_status_response.json",
        );
    }

    #[test]
    fn fixture_init_upload_request() {
        if !fixtures_dir().join("init_upload_request.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::messages::InitUploadRequest>(
            "init_upload_request.json",
        );
    }

    #[test]
    fn fixture_upload_chunk_request() {
        if !fixtures_dir().join("upload_chunk_request.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::messages::UploadChunkRequest>(
            "upload_chunk_request.json",
        );
    }

    #[test]
    fn fixture_telemetry_data() {
        if !fixtures_dir().join("telemetry_data.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::telemetry::TelemetryData>("telemetry_data.json");
    }

    #[test]
    fn fixture_console_log_batch() {
        if !fixtures_dir().join("console_log_batch.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::console_log::ConsoleLogBatch>(
            "console_log_batch.json",
        );
    }

    #[test]
    fn fixture_game_log_wrapper_status() {
        if !fixtures_dir().join("game_log_wrapper_status.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::telemetry::GameLogWrapperStatusEvent>(
            "game_log_wrapper_status.json",
        );
    }

    #[test]
    fn fixture_shortcut_config() {
        if !fixtures_dir().join("shortcut_config.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::ShortcutConfig>("shortcut_config.json");
    }

    #[test]
    fn fixture_shortcut_info() {
        if !fixtures_dir().join("shortcut_info.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::ShortcutInfo>("shortcut_info.json");
    }

    #[test]
    fn fixture_operation_event() {
        if !fixtures_dir().join("operation_event.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::messages::OperationEvent>("operation_event.json");
    }

    // --- Upload response fixtures ---

    #[test]
    fn fixture_init_upload_response_full() {
        if !fixtures_dir()
            .join("init_upload_response_full.json")
            .exists()
        {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::messages::InitUploadResponseFull>(
            "init_upload_response_full.json",
        );
    }

    #[test]
    fn legacy_init_upload_response_full_no_tcp() {
        let json = load_fixture("init_upload_response_full_legacy.json");
        let resp: capydeploy_protocol::messages::InitUploadResponseFull =
            serde_json::from_value(json).unwrap();
        assert_eq!(resp.upload_id, "upload-99999");
        assert_eq!(resp.chunk_size, 1048576);
        assert!(
            resp.resume_from.is_none(),
            "missing field should default to None"
        );
        assert!(
            resp.tcp_port.is_none(),
            "missing tcp_port should default to None"
        );
        assert!(
            resp.tcp_token.is_none(),
            "missing tcp_token should default to None"
        );
    }

    // --- Data channel fixtures ---

    #[test]
    fn fixture_agent_status_with_capabilities() {
        if !fixtures_dir()
            .join("agent_status_response_with_capabilities.json")
            .exists()
        {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::messages::AgentStatusResponse>(
            "agent_status_response_with_capabilities.json",
        );
    }

    #[test]
    fn fixture_data_channel_ready_event() {
        if !fixtures_dir()
            .join("data_channel_ready_event.json")
            .exists()
        {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        roundtrip_test::<capydeploy_protocol::messages::DataChannelReadyEvent>(
            "data_channel_ready_event.json",
        );
    }

    // --- Backward compatibility: legacy JSON without protocolVersion ---

    #[test]
    fn legacy_agent_status_no_capabilities() {
        // Legacy agents don't send capabilities — should default to empty vec.
        let json = r#"{
            "name": "OldAgent",
            "version": "0.5.0",
            "platform": "steamdeck",
            "acceptConnections": true,
            "telemetryEnabled": false,
            "telemetryInterval": 2,
            "consoleLogEnabled": false,
            "protocolVersion": 1
        }"#;
        let resp: capydeploy_protocol::messages::AgentStatusResponse =
            serde_json::from_str(json).unwrap();
        assert!(
            resp.capabilities.is_empty(),
            "missing capabilities should default to empty vec"
        );
    }

    #[test]
    fn legacy_hub_connected_no_protocol_version() {
        let json = r#"{
            "name": "OldHub",
            "version": "0.5.0",
            "platform": "linux",
            "hubId": "hub-old",
            "token": "tok"
        }"#;
        let req: capydeploy_protocol::messages::HubConnectedRequest =
            serde_json::from_str(json).unwrap();
        assert_eq!(req.protocol_version, 0, "missing field should default to 0");
    }

    #[test]
    fn legacy_agent_status_no_protocol_version() {
        let json = r#"{
            "name": "OldAgent",
            "version": "0.5.0",
            "platform": "steamdeck",
            "acceptConnections": true,
            "telemetryEnabled": false,
            "telemetryInterval": 2,
            "consoleLogEnabled": false
        }"#;
        let resp: capydeploy_protocol::messages::AgentStatusResponse =
            serde_json::from_str(json).unwrap();
        assert_eq!(
            resp.protocol_version, 0,
            "missing field should default to 0"
        );
    }

    // --- Steam crate tests ---

    #[test]
    fn fixture_steam_app_id() {
        if !fixtures_dir().join("steam_app_id.json").exists() {
            eprintln!("SKIP: fixture not generated yet");
            return;
        }
        let fixture = load_fixture("steam_app_id.json");
        let exe = fixture["exe"].as_str().unwrap();
        let name = fixture["name"].as_str().unwrap();
        let expected = fixture["appId"].as_u64().unwrap() as u32;
        let got = capydeploy_steam::generate_app_id(exe, name);
        assert_eq!(
            got, expected,
            "AppID mismatch: exe={exe}, name={name}, Go={expected}, Rust={got}"
        );
    }
}
