---
name: go_dev
description: Go 1.23 backend development for Wails apps.
---
# Go 1.23 Backend (Wails)

- **Ver**: Go 1.23+
- **Files**: `snake_case.go`; Packages: `lowercase`
- **Role**: Backend logic, Wails bindings, NO UI code

## Go Idioms
- **Errors**: Return `error` as last value. Wrap with `fmt.Errorf("context: %w", err)`
- **Defer**: Use for cleanup (Close, Unlock). Defer immediately after resource acquisition.
- **Interfaces**: Accept interfaces, return structs. Keep interfaces small.
- **Concurrency**: Channels for communication. `sync.Mutex` for shared state. Context for cancellation.

## Wails Bindings
- **Expose functions**: Public methods on `App` struct are auto-bound.
- **Return types**: Use simple types or structs (serialized to JSON).
- **Errors**: Return `error` as second value, frontend receives as rejected promise.
- **Events**: Use `runtime.EventsEmit()` for Go->Frontend notifications.
- **Context**: Store `context.Context` from `startup()` for runtime calls.

```go
// Example binding
func (a *App) GetDevices() ([]Device, error) {
    return a.deviceManager.List()
}

// Example event emission
runtime.EventsEmit(a.ctx, "upload:progress", UploadProgress{Percent: 50})
```

## SSH/SFTP (this project)
- **Connections**: Reuse clients when possible. Close in defer.
- **Timeouts**: Always set connection/operation timeouts.
- **Errors**: Handle network errors gracefully. Return user-friendly messages to frontend.
- **Paths**: Use `path.Join` for remote paths (Unix-style even from Windows).

## Performance
- Use `strings.Builder` for string concatenation.
- Preallocate slices when size is known: `make([]T, 0, capacity)`.
- Profile with `pprof` before optimizing.
- Avoid blocking Wails event loop with long operations (use goroutines).
