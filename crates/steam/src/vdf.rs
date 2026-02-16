use std::fs;
use std::path::Path;

use capydeploy_protocol::ShortcutInfo;

use crate::SteamError;

/// Binary VDF type markers used in shortcuts.vdf.
const VDF_TYPE_OBJECT: u8 = 0x00;
const VDF_TYPE_STRING: u8 = 0x01;
const VDF_TYPE_INT32: u8 = 0x02;
const VDF_TYPE_END: u8 = 0x08;

/// Parses a binary VDF shortcuts file and returns shortcut info.
pub fn load_shortcuts_vdf(path: &Path) -> Result<Vec<ShortcutInfo>, SteamError> {
    let data = fs::read(path)
        .map_err(|e| SteamError::Vdf(format!("failed to read shortcuts file: {e}")))?;
    parse_shortcuts_vdf(&data)
}

/// Parses binary VDF data into shortcuts.
fn parse_shortcuts_vdf(data: &[u8]) -> Result<Vec<ShortcutInfo>, SteamError> {
    if data.len() < 3 {
        return Err(SteamError::Vdf("shortcuts file too small".into()));
    }

    let mut pos = 0;

    // Expect root object marker + "shortcuts" + null
    if data[pos] != VDF_TYPE_OBJECT {
        return Err(SteamError::Vdf(format!(
            "expected object marker at start, got 0x{:02x}",
            data[pos]
        )));
    }
    pos += 1;

    let (name, new_pos) = read_string(data, pos)?;
    pos = new_pos;

    if name != "shortcuts" {
        return Err(SteamError::Vdf(format!(
            "expected root key 'shortcuts', got '{name}'"
        )));
    }

    let mut shortcuts = Vec::new();

    while pos < data.len() {
        if data[pos] == VDF_TYPE_END {
            break;
        }

        if data[pos] != VDF_TYPE_OBJECT {
            return Err(SteamError::Vdf(format!(
                "expected object marker for shortcut at pos {pos}, got 0x{:02x}",
                data[pos]
            )));
        }
        pos += 1;

        // Skip the index key (e.g., "0", "1", "2")
        let (_, new_pos) = read_string(data, pos)?;
        pos = new_pos;

        let (sc, new_pos) = parse_shortcut_entry(data, pos)?;
        pos = new_pos;

        shortcuts.push(sc);
    }

    Ok(shortcuts)
}

/// Parses a single shortcut entry from VDF data.
fn parse_shortcut_entry(data: &[u8], mut pos: usize) -> Result<(ShortcutInfo, usize), SteamError> {
    let mut sc = ShortcutInfo {
        app_id: 0,
        name: String::new(),
        exe: String::new(),
        start_dir: String::new(),
        launch_options: String::new(),
        tags: vec![],
        last_played: 0,
    };

    while pos < data.len() {
        if data[pos] == VDF_TYPE_END {
            pos += 1;
            return Ok((sc, pos));
        }

        let type_byte = data[pos];
        pos += 1;

        let (key, new_pos) = read_string(data, pos)?;
        pos = new_pos;

        match type_byte {
            VDF_TYPE_STRING => {
                let (val, new_pos) = read_string(data, pos)?;
                pos = new_pos;

                match key.as_str() {
                    "AppName" | "appname" => sc.name = val,
                    "Exe" | "exe" => sc.exe = val,
                    "StartDir" | "startdir" | "StartDir\x00" => sc.start_dir = val,
                    "LaunchOptions" | "launchoptions" => sc.launch_options = val,
                    _ => {}
                }
            }
            VDF_TYPE_INT32 => {
                if pos + 4 > data.len() {
                    return Err(SteamError::Vdf(format!(
                        "unexpected end of data reading int32 for '{key}'"
                    )));
                }
                let val =
                    u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                pos += 4;

                match key.as_str() {
                    "appid" => sc.app_id = val,
                    "LastPlayTime" | "lastplaytime" => sc.last_played = val as i64,
                    _ => {}
                }
            }
            VDF_TYPE_OBJECT => {
                if key == "tags" {
                    let (tags, new_pos) = parse_tags(data, pos)?;
                    pos = new_pos;
                    sc.tags = tags;
                } else {
                    let new_pos = skip_object(data, pos)?;
                    pos = new_pos;
                }
            }
            _ => {
                return Err(SteamError::Vdf(format!(
                    "unknown type marker 0x{type_byte:02x} for key '{key}' at pos {pos}"
                )));
            }
        }
    }

    Err(SteamError::Vdf(
        "unexpected end of data in shortcut entry".into(),
    ))
}

/// Parses the tags nested object into a string vector.
fn parse_tags(data: &[u8], mut pos: usize) -> Result<(Vec<String>, usize), SteamError> {
    let mut tags = Vec::new();

    while pos < data.len() {
        if data[pos] == VDF_TYPE_END {
            pos += 1;
            return Ok((tags, pos));
        }

        let type_byte = data[pos];
        pos += 1;

        // Read key (tag index like "0", "1", etc.)
        let (_, new_pos) = read_string(data, pos)?;
        pos = new_pos;

        match type_byte {
            VDF_TYPE_STRING => {
                let (val, new_pos) = read_string(data, pos)?;
                pos = new_pos;
                tags.push(val);
            }
            VDF_TYPE_INT32 => {
                pos += 4;
            }
            VDF_TYPE_OBJECT => {
                pos = skip_object(data, pos)?;
            }
            _ => {}
        }
    }

    Ok((tags, pos))
}

/// Skips an entire nested VDF object.
fn skip_object(data: &[u8], mut pos: usize) -> Result<usize, SteamError> {
    while pos < data.len() {
        if data[pos] == VDF_TYPE_END {
            pos += 1;
            return Ok(pos);
        }

        let type_byte = data[pos];
        pos += 1;

        // Skip key name
        let (_, new_pos) = read_string(data, pos)?;
        pos = new_pos;

        match type_byte {
            VDF_TYPE_STRING => {
                let (_, new_pos) = read_string(data, pos)?;
                pos = new_pos;
            }
            VDF_TYPE_INT32 => {
                if pos + 4 > data.len() {
                    return Err(SteamError::Vdf("unexpected end of data".into()));
                }
                pos += 4;
            }
            VDF_TYPE_OBJECT => {
                pos = skip_object(data, pos)?;
            }
            _ => {
                return Err(SteamError::Vdf(format!(
                    "unknown type 0x{type_byte:02x} while skipping"
                )));
            }
        }
    }

    Err(SteamError::Vdf(
        "unexpected end of data while skipping object".into(),
    ))
}

/// Reads a null-terminated string from data starting at pos.
fn read_string(data: &[u8], pos: usize) -> Result<(String, usize), SteamError> {
    let start = pos;
    let mut i = pos;
    while i < data.len() {
        if data[i] == 0x00 {
            let s = String::from_utf8_lossy(&data[start..i]).into_owned();
            return Ok((s, i + 1));
        }
        i += 1;
    }
    Err(SteamError::Vdf(format!(
        "unterminated string starting at pos {start}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal valid shortcuts.vdf binary.
    fn build_test_vdf(shortcuts: &[(&str, &str, &str, u32)]) -> Vec<u8> {
        let mut data = Vec::new();
        // Root: \x00 "shortcuts" \x00
        data.push(VDF_TYPE_OBJECT);
        data.extend_from_slice(b"shortcuts\x00");

        for (i, (name, exe, start_dir, app_id)) in shortcuts.iter().enumerate() {
            // Entry: \x00 "<index>" \x00
            data.push(VDF_TYPE_OBJECT);
            data.extend_from_slice(i.to_string().as_bytes());
            data.push(0x00);

            // appid (int32)
            data.push(VDF_TYPE_INT32);
            data.extend_from_slice(b"appid\x00");
            data.extend_from_slice(&app_id.to_le_bytes());

            // AppName (string)
            data.push(VDF_TYPE_STRING);
            data.extend_from_slice(b"AppName\x00");
            data.extend_from_slice(name.as_bytes());
            data.push(0x00);

            // Exe (string)
            data.push(VDF_TYPE_STRING);
            data.extend_from_slice(b"Exe\x00");
            data.extend_from_slice(exe.as_bytes());
            data.push(0x00);

            // StartDir (string)
            data.push(VDF_TYPE_STRING);
            data.extend_from_slice(b"StartDir\x00");
            data.extend_from_slice(start_dir.as_bytes());
            data.push(0x00);

            // End of entry
            data.push(VDF_TYPE_END);
        }

        // End of root
        data.push(VDF_TYPE_END);
        data
    }

    #[test]
    fn parse_empty_shortcuts() {
        let data = build_test_vdf(&[]);
        let shortcuts = parse_shortcuts_vdf(&data).unwrap();
        assert!(shortcuts.is_empty());
    }

    #[test]
    fn parse_single_shortcut() {
        let data = build_test_vdf(&[("Test Game", "/usr/bin/game", "/home/user", 12345)]);
        let shortcuts = parse_shortcuts_vdf(&data).unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].name, "Test Game");
        assert_eq!(shortcuts[0].exe, "/usr/bin/game");
        assert_eq!(shortcuts[0].start_dir, "/home/user");
        assert_eq!(shortcuts[0].app_id, 12345);
    }

    #[test]
    fn parse_multiple_shortcuts() {
        let data = build_test_vdf(&[
            ("Game A", "/bin/a", "/home", 100),
            ("Game B", "/bin/b", "/home", 200),
            ("Game C", "/bin/c", "/home", 300),
        ]);
        let shortcuts = parse_shortcuts_vdf(&data).unwrap();
        assert_eq!(shortcuts.len(), 3);
        assert_eq!(shortcuts[0].name, "Game A");
        assert_eq!(shortcuts[2].app_id, 300);
    }

    #[test]
    fn parse_shortcut_with_tags() {
        let mut data = Vec::new();
        data.push(VDF_TYPE_OBJECT);
        data.extend_from_slice(b"shortcuts\x00");

        // Single entry
        data.push(VDF_TYPE_OBJECT);
        data.extend_from_slice(b"0\x00");

        data.push(VDF_TYPE_INT32);
        data.extend_from_slice(b"appid\x00");
        data.extend_from_slice(&42u32.to_le_bytes());

        data.push(VDF_TYPE_STRING);
        data.extend_from_slice(b"AppName\x00");
        data.extend_from_slice(b"Tagged Game\x00");

        // Tags object
        data.push(VDF_TYPE_OBJECT);
        data.extend_from_slice(b"tags\x00");
        data.push(VDF_TYPE_STRING);
        data.extend_from_slice(b"0\x00");
        data.extend_from_slice(b"RPG\x00");
        data.push(VDF_TYPE_STRING);
        data.extend_from_slice(b"1\x00");
        data.extend_from_slice(b"Action\x00");
        data.push(VDF_TYPE_END); // end tags

        data.push(VDF_TYPE_END); // end entry
        data.push(VDF_TYPE_END); // end root

        let shortcuts = parse_shortcuts_vdf(&data).unwrap();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].tags, vec!["RPG", "Action"]);
    }

    #[test]
    fn reject_too_small() {
        assert!(parse_shortcuts_vdf(&[0x00, 0x00]).is_err());
    }

    #[test]
    fn reject_wrong_root_key() {
        let mut data = vec![VDF_TYPE_OBJECT];
        data.extend_from_slice(b"wrong\x00");
        data.push(VDF_TYPE_END);
        assert!(parse_shortcuts_vdf(&data).is_err());
    }

    #[test]
    fn read_string_basic() {
        let data = b"hello\x00world";
        let (s, pos) = read_string(data, 0).unwrap();
        assert_eq!(s, "hello");
        assert_eq!(pos, 6);
    }

    #[test]
    fn read_string_unterminated() {
        let data = b"no null";
        assert!(read_string(data, 0).is_err());
    }
}
