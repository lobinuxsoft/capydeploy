package steam

import (
	"encoding/binary"
	"fmt"
	"os"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// Binary VDF type markers used in shortcuts.vdf.
const (
	vdfTypeObject byte = 0x00
	vdfTypeString byte = 0x01
	vdfTypeInt32  byte = 0x02
	vdfTypeEnd    byte = 0x08
)

// LoadShortcutsVDF parses a binary VDF shortcuts file and returns shortcut info.
// The binary VDF format uses type markers (\x00=object, \x01=string, \x02=int32, \x08=end).
func LoadShortcutsVDF(path string) ([]protocol.ShortcutInfo, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read shortcuts file: %w", err)
	}

	return parseShortcutsVDF(data)
}

// parseShortcutsVDF parses the binary VDF data into shortcuts.
func parseShortcutsVDF(data []byte) ([]protocol.ShortcutInfo, error) {
	if len(data) < 3 {
		return nil, fmt.Errorf("shortcuts file too small")
	}

	pos := 0

	// Expect root object marker + "shortcuts" + null
	if data[pos] != vdfTypeObject {
		return nil, fmt.Errorf("expected object marker at start, got 0x%02x", data[pos])
	}
	pos++

	name, newPos, err := readString(data, pos)
	if err != nil {
		return nil, fmt.Errorf("failed to read root name: %w", err)
	}
	pos = newPos

	if name != "shortcuts" {
		return nil, fmt.Errorf("expected root key 'shortcuts', got %q", name)
	}

	var shortcuts []protocol.ShortcutInfo

	// Read each shortcut entry (object type with numeric key)
	for pos < len(data) {
		if data[pos] == vdfTypeEnd {
			break
		}

		if data[pos] != vdfTypeObject {
			return nil, fmt.Errorf("expected object marker for shortcut at pos %d, got 0x%02x", pos, data[pos])
		}
		pos++

		// Skip the index key (e.g. "0", "1", "2")
		_, newPos, err := readString(data, pos)
		if err != nil {
			return nil, fmt.Errorf("failed to read shortcut index: %w", err)
		}
		pos = newPos

		sc, newPos2, err := parseShortcutEntry(data, pos)
		if err != nil {
			return nil, fmt.Errorf("failed to parse shortcut: %w", err)
		}
		pos = newPos2

		shortcuts = append(shortcuts, sc)
	}

	return shortcuts, nil
}

// parseShortcutEntry parses a single shortcut entry from the VDF data.
func parseShortcutEntry(data []byte, pos int) (protocol.ShortcutInfo, int, error) {
	var sc protocol.ShortcutInfo

	for pos < len(data) {
		if data[pos] == vdfTypeEnd {
			pos++ // consume end marker
			return sc, pos, nil
		}

		typeByte := data[pos]
		pos++

		key, newPos, err := readString(data, pos)
		if err != nil {
			return sc, pos, fmt.Errorf("failed to read field key: %w", err)
		}
		pos = newPos

		switch typeByte {
		case vdfTypeString:
			val, newPos, err := readString(data, pos)
			if err != nil {
				return sc, pos, fmt.Errorf("failed to read string value for %q: %w", key, err)
			}
			pos = newPos

			switch key {
			case "AppName", "appname":
				sc.Name = val
			case "Exe", "exe":
				sc.Exe = val
			case "StartDir", "StartDir\x00", "startdir":
				sc.StartDir = val
			case "LaunchOptions", "launchoptions":
				sc.LaunchOptions = val
			}

		case vdfTypeInt32:
			if pos+4 > len(data) {
				return sc, pos, fmt.Errorf("unexpected end of data reading int32 for %q", key)
			}
			val := binary.LittleEndian.Uint32(data[pos : pos+4])
			pos += 4

			switch key {
			case "appid":
				sc.AppID = val
			case "LastPlayTime", "lastplaytime":
				sc.LastPlayed = int64(val)
			}

		case vdfTypeObject:
			// Nested object (e.g., "tags") — parse to extract tags
			if key == "tags" {
				tags, newPos, err := parseTags(data, pos)
				if err != nil {
					return sc, pos, fmt.Errorf("failed to parse tags: %w", err)
				}
				pos = newPos
				sc.Tags = tags
			} else {
				// Skip unknown nested objects
				newPos, err := skipObject(data, pos)
				if err != nil {
					return sc, pos, fmt.Errorf("failed to skip object %q: %w", key, err)
				}
				pos = newPos
			}

		default:
			return sc, pos, fmt.Errorf("unknown type marker 0x%02x for key %q at pos %d", typeByte, key, pos)
		}
	}

	return sc, pos, fmt.Errorf("unexpected end of data in shortcut entry")
}

// parseTags parses the tags nested object into a string slice.
func parseTags(data []byte, pos int) ([]string, int, error) {
	var tags []string

	for pos < len(data) {
		if data[pos] == vdfTypeEnd {
			pos++
			return tags, pos, nil
		}

		typeByte := data[pos]
		pos++

		// Read key (tag index like "0", "1", etc.)
		_, newPos, err := readString(data, pos)
		if err != nil {
			return nil, pos, err
		}
		pos = newPos

		if typeByte == vdfTypeString {
			val, newPos, err := readString(data, pos)
			if err != nil {
				return nil, pos, err
			}
			pos = newPos
			tags = append(tags, val)
		} else if typeByte == vdfTypeInt32 {
			pos += 4
		} else if typeByte == vdfTypeObject {
			newPos, err := skipObject(data, pos)
			if err != nil {
				return nil, pos, err
			}
			pos = newPos
		}
	}

	return tags, pos, nil
}

// skipObject skips an entire nested VDF object.
func skipObject(data []byte, pos int) (int, error) {
	for pos < len(data) {
		if data[pos] == vdfTypeEnd {
			pos++
			return pos, nil
		}

		typeByte := data[pos]
		pos++

		// Skip key name
		_, newPos, err := readString(data, pos)
		if err != nil {
			return pos, err
		}
		pos = newPos

		switch typeByte {
		case vdfTypeString:
			_, newPos, err := readString(data, pos)
			if err != nil {
				return pos, err
			}
			pos = newPos
		case vdfTypeInt32:
			if pos+4 > len(data) {
				return pos, fmt.Errorf("unexpected end of data")
			}
			pos += 4
		case vdfTypeObject:
			newPos, err := skipObject(data, pos)
			if err != nil {
				return pos, err
			}
			pos = newPos
		default:
			return pos, fmt.Errorf("unknown type 0x%02x while skipping", typeByte)
		}
	}

	return pos, fmt.Errorf("unexpected end of data while skipping object")
}

// readString reads a null-terminated string from data starting at pos.
func readString(data []byte, pos int) (string, int, error) {
	start := pos
	for pos < len(data) {
		if data[pos] == 0x00 {
			return string(data[start:pos]), pos + 1, nil
		}
		pos++
	}
	return "", pos, fmt.Errorf("unterminated string starting at pos %d", start)
}
