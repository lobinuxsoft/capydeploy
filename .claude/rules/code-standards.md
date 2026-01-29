---
trigger: glob
priority: critical
---

# Go Backend
Naming:
- Packages: lowercase, single word (config, device, shortcuts)
- Exported: PascalCase (DeviceClient, UploadGame)
- Unexported: camelCase (parseConfig, buildPath)
- Comments: EN.

Guidelines: GoDoc for public APIs. Error wrapping with context. Defer for cleanup.

# Svelte/TypeScript Frontend
Naming:
- Components: PascalCase (`DeviceList.svelte`, `ArtworkGrid.svelte`)
- Files: kebab-case for non-components (`api-client.ts`, `types.ts`)
- Variables/Functions: camelCase
- Types/Interfaces: PascalCase
- CSS classes: Tailwind utilities, kebab-case for custom

Structure:
- `frontend/src/lib/` - Reusable components, utilities
- `frontend/src/routes/` or `frontend/src/App.svelte` - Main views
- `frontend/src/lib/components/ui/` - shadcn-svelte components

# General
Arch:
- Go: `internal/` for private packages, Wails bindings in `app.go`
- Frontend: Component composition, props down / events up
- State: Svelte stores for global state, props for local

Tests: Go table-driven tests. Vitest for frontend. Flag missing tests.

Design: SOLID, DRY, KISS > Abstract, YAGNI.

Security:
- Never log passwords/SSH keys
- Config file 0600 permissions
- Validate all user input paths
- Sanitize data passed between Go<->JS
