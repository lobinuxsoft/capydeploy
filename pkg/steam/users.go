package steam

import (
	"os"
	"strconv"
)

// User represents a Steam user with shortcuts.
type User struct {
	ID          string `json:"id"`
	HasShortcuts bool  `json:"hasShortcuts"`
}

// GetUsers returns a list of Steam users from the userdata directory.
func GetUsers() ([]User, error) {
	paths, err := NewPaths()
	if err != nil {
		return nil, err
	}
	return GetUsersWithPaths(paths)
}

// GetUsersWithPaths returns users using the provided Paths instance.
func GetUsersWithPaths(paths *Paths) ([]User, error) {
	userDataDir := paths.UserDataDir()

	entries, err := os.ReadDir(userDataDir)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, ErrSteamNotFound
		}
		return nil, err
	}

	var users []User
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}

		// Verify it's a numeric user ID
		name := entry.Name()
		if _, err := strconv.ParseUint(name, 10, 64); err != nil {
			continue
		}

		// Skip "0" directory - it's a temporary Steam directory, not a real user
		if name == "0" {
			continue
		}

		users = append(users, User{
			ID:          name,
			HasShortcuts: paths.HasShortcuts(name),
		})
	}

	return users, nil
}

// GetFirstUserWithShortcuts returns the first user that has shortcuts.
func GetFirstUserWithShortcuts() (*User, error) {
	users, err := GetUsers()
	if err != nil {
		return nil, err
	}

	for _, u := range users {
		if u.HasShortcuts {
			return &u, nil
		}
	}

	// If no user has shortcuts, return the first user
	if len(users) > 0 {
		return &users[0], nil
	}

	return nil, ErrUserNotFound
}

// UserIDToUint32 converts a string user ID to uint32.
func UserIDToUint32(userID string) (uint32, error) {
	id, err := strconv.ParseUint(userID, 10, 32)
	if err != nil {
		return 0, err
	}
	return uint32(id), nil
}

// Uint32ToUserID converts a uint32 user ID to string.
func Uint32ToUserID(userID uint32) string {
	return strconv.FormatUint(uint64(userID), 10)
}
