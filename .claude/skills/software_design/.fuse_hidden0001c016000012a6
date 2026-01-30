---
name: software_design
description: SOLID/Patterns for Go applications.
---
# Architecture

- **Comp > Inherit**: Go has no inheritance. Use composition and embedding.
- **Package Design**: Each package = one responsibility. `internal/` for private implementation.

## SOLID in Go

- **SRP**: One package = one purpose. `config/` handles config, `device/` handles SSH.
- **OCP**: Use interfaces to extend behavior without modifying existing code.
- **LSP**: Interface implementations must be substitutable.
- **ISP**: Small, focused interfaces. `io.Reader` not `io.ReadWriteCloser` when only reading.
- **DIP**: Accept interfaces in function params. Inject dependencies, don't create inside.

## Patterns

- **Repository**: Wrap data access (config.Store, device.Repository).
- **Factory**: Functions returning interfaces (`NewClient() Client`).
- **Strategy**: Interface + multiple implementations (auth methods: password vs key).
- **Observer**: Channels or callback functions for async notifications.
- **Builder**: For complex object construction with many optional params.

## Project Structure (bazzite-devkit)
```
cmd/bazzite-devkit/     # Entry point only
internal/
  config/               # App configuration, persistence
  device/               # SSH/SFTP client, device discovery
  ui/                   # Fyne GUI components
  shortcuts/            # Steam shortcut management
```

## Error Handling
- Define custom error types for domain errors.
- Use `errors.Is/As` for error checking.
- Wrap errors with context at each layer.
- Log at the top level, return errors from lower levels.
