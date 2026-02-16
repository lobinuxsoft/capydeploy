use std::fs;

use serde::{Deserialize, Serialize};

use crate::SteamError;
use crate::paths::Paths;

/// A Steam user with shortcut information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub has_shortcuts: bool,
}

/// Returns a list of Steam users from the userdata directory.
pub fn get_users() -> Result<Vec<User>, SteamError> {
    let paths = Paths::new()?;
    get_users_with_paths(&paths)
}

/// Returns users using the provided `Paths` instance.
pub fn get_users_with_paths(paths: &Paths) -> Result<Vec<User>, SteamError> {
    let user_data_dir = paths.user_data_dir();

    let entries = fs::read_dir(&user_data_dir).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SteamError::NotFound
        } else {
            SteamError::Io(e.to_string())
        }
    })?;

    let mut users = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| SteamError::Io(e.to_string()))?;

        if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            continue;
        }

        let name = entry.file_name();
        let name = name.to_string_lossy();

        // Verify it's a numeric user ID
        if name.parse::<u64>().is_err() {
            continue;
        }

        // Skip "0" directory — temporary Steam directory, not a real user
        if name == "0" {
            continue;
        }

        let has_shortcuts = paths.has_shortcuts(&name);
        users.push(User {
            id: name.into_owned(),
            has_shortcuts,
        });
    }

    Ok(users)
}

/// Returns the first user that has shortcuts, or the first user if none do.
pub fn get_first_user_with_shortcuts() -> Result<Option<User>, SteamError> {
    let users = get_users()?;

    for u in &users {
        if u.has_shortcuts {
            return Ok(Some(u.clone()));
        }
    }

    Ok(users.into_iter().next())
}

/// Converts a string user ID to u32.
pub fn user_id_to_u32(user_id: &str) -> Result<u32, std::num::ParseIntError> {
    user_id.parse::<u32>()
}

/// Converts a u32 user ID to string.
pub fn u32_to_user_id(user_id: u32) -> String {
    user_id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn user_id_conversion_roundtrip() {
        assert_eq!(user_id_to_u32("12345").unwrap(), 12345);
        assert_eq!(u32_to_user_id(12345), "12345");
    }

    #[test]
    fn user_id_invalid() {
        assert!(user_id_to_u32("not_a_number").is_err());
    }

    #[test]
    fn get_users_with_temp_dir() {
        let tmp = std::env::temp_dir().join("capydeploy_test_users");
        let _ = fs::remove_dir_all(&tmp);

        // Create fake Steam directory
        let userdata = tmp.join("userdata");
        fs::create_dir_all(userdata.join("12345").join("config")).unwrap();
        fs::create_dir_all(userdata.join("67890").join("config")).unwrap();
        fs::create_dir_all(userdata.join("0").join("config")).unwrap();

        // Create shortcuts.vdf for user 12345
        fs::write(
            userdata.join("12345").join("config").join("shortcuts.vdf"),
            b"test",
        )
        .unwrap();

        let paths = Paths::with_base(&tmp);
        let users = get_users_with_paths(&paths).unwrap();

        // Should have 2 users (skipping "0")
        assert_eq!(users.len(), 2);

        let user_12345 = users.iter().find(|u| u.id == "12345").unwrap();
        assert!(user_12345.has_shortcuts);

        let user_67890 = users.iter().find(|u| u.id == "67890").unwrap();
        assert!(!user_67890.has_shortcuts);

        // No user "0"
        assert!(users.iter().all(|u| u.id != "0"));

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn user_json_field_names() {
        let user = User {
            id: "123".into(),
            has_shortcuts: true,
        };
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("\"hasShortcuts\""));
    }
}
