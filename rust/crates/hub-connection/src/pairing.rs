//! Hub-Agent pairing flow.
//!
//! Manages pairing code exchange and token persistence
//! for authorized Agent connections.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use tracing::debug;

/// Errors from pairing operations.
#[derive(Debug, thiserror::Error)]
pub enum PairingError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Persistent token store for paired Agents.
///
/// Tokens are cached in memory and persisted to a JSON file.
pub struct TokenStore {
    path: PathBuf,
    tokens: RwLock<HashMap<String, String>>,
}

impl TokenStore {
    /// Creates a new token store, loading existing tokens from disk.
    pub fn new(path: PathBuf) -> Result<Self, PairingError> {
        let tokens = load_tokens(&path)?;
        Ok(Self {
            path,
            tokens: RwLock::new(tokens),
        })
    }

    /// Returns the token for an Agent, if any.
    pub fn get_token(&self, agent_id: &str) -> Option<String> {
        self.tokens.read().unwrap().get(agent_id).cloned()
    }

    /// Saves a token for an Agent.
    pub fn save_token(&self, agent_id: &str, token: &str) -> Result<(), PairingError> {
        {
            let mut map = self.tokens.write().unwrap();
            map.insert(agent_id.to_string(), token.to_string());
        }
        self.persist()
    }

    /// Removes a token for an Agent.
    pub fn remove_token(&self, agent_id: &str) -> Result<(), PairingError> {
        {
            let mut map = self.tokens.write().unwrap();
            map.remove(agent_id);
        }
        self.persist()
    }

    /// Returns all stored Agent IDs.
    pub fn agent_ids(&self) -> Vec<String> {
        self.tokens.read().unwrap().keys().cloned().collect()
    }

    /// Writes the current tokens to disk.
    fn persist(&self) -> Result<(), PairingError> {
        let map = self.tokens.read().unwrap();
        let json = serde_json::to_string_pretty(&*map)?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, json)?;
        debug!("persisted {} token(s) to {:?}", map.len(), self.path);
        Ok(())
    }
}

/// Loads tokens from a JSON file on disk.
fn load_tokens(path: &Path) -> Result<HashMap<String, String>, PairingError> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let data = std::fs::read_to_string(path)?;
    let tokens: HashMap<String, String> = serde_json::from_str(&data)?;
    debug!("loaded {} token(s) from {:?}", tokens.len(), path);
    Ok(tokens)
}

/// Returns the default token store path.
pub fn default_token_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("capydeploy").join("hub").join("tokens.json"))
}

/// Returns the platform-specific config directory.
fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
    }

    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".config"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> (tempfile::TempDir, TokenStore) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tokens.json");
        let store = TokenStore::new(path).unwrap();
        (tmp, store)
    }

    #[test]
    fn new_store_empty() {
        let (_tmp, store) = test_store();
        assert!(store.agent_ids().is_empty());
        assert!(store.get_token("agent-1").is_none());
    }

    #[test]
    fn save_and_get_token() {
        let (_tmp, store) = test_store();
        store.save_token("agent-1", "token-abc").unwrap();
        assert_eq!(store.get_token("agent-1").unwrap(), "token-abc");
    }

    #[test]
    fn remove_token() {
        let (_tmp, store) = test_store();
        store.save_token("agent-1", "token-abc").unwrap();
        store.remove_token("agent-1").unwrap();
        assert!(store.get_token("agent-1").is_none());
    }

    #[test]
    fn persist_and_reload() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tokens.json");

        {
            let store = TokenStore::new(path.clone()).unwrap();
            store.save_token("agent-1", "tok-1").unwrap();
            store.save_token("agent-2", "tok-2").unwrap();
        }

        // Reload from disk.
        let store2 = TokenStore::new(path).unwrap();
        assert_eq!(store2.get_token("agent-1").unwrap(), "tok-1");
        assert_eq!(store2.get_token("agent-2").unwrap(), "tok-2");
        assert_eq!(store2.agent_ids().len(), 2);
    }

    #[test]
    fn overwrite_token() {
        let (_tmp, store) = test_store();
        store.save_token("agent-1", "old-token").unwrap();
        store.save_token("agent-1", "new-token").unwrap();
        assert_eq!(store.get_token("agent-1").unwrap(), "new-token");
    }

    #[test]
    fn agent_ids_returns_all() {
        let (_tmp, store) = test_store();
        store.save_token("a", "1").unwrap();
        store.save_token("b", "2").unwrap();
        store.save_token("c", "3").unwrap();

        let mut ids = store.agent_ids();
        ids.sort();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let path = PathBuf::from("/tmp/nonexistent_capydeploy_test_tokens.json");
        let tokens = load_tokens(&path).unwrap();
        assert!(tokens.is_empty());
    }
}
