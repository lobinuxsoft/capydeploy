# Contributing to CapyDeploy

Thank you for your interest in contributing! This document outlines our development workflow and standards.

## Quick Reference

| Item | Value |
|------|-------|
| PRs target | `development` branch |
| Commit language | Spanish |
| Commit format | [Conventional Commits](https://www.conventionalcommits.org/) |
| Code comments | English |

## Issue-First Development

**Always create an issue before coding.**

```
Create Issue → Create Branch → Develop → PR to development → Close Issue
```

This ensures work is tracked, discussed, and properly scoped before implementation begins.

## Branch Naming

Create branches from issues using this pattern:

```
feature/issue-XX-short-description
fix/issue-XX-short-description
docs/issue-XX-short-description
refactor/issue-XX-short-description
```

Example: `feature/issue-42-add-artwork-cache`

## Commit Messages

Write commits in **Spanish** using Conventional Commits format:

```
feat: agregar caché de artwork
fix: corregir conexión WebSocket en Windows
docs: actualizar guía de instalación
refactor: simplificar protocolo de transferencia
test: agregar tests para shortcuts
chore: actualizar dependencias
```

### Types

| Type | Use for |
|------|---------|
| `feat` | New features |
| `fix` | Bug fixes |
| `docs` | Documentation only |
| `refactor` | Code changes that neither fix bugs nor add features |
| `test` | Adding or updating tests |
| `chore` | Maintenance tasks |
| `build` | Build system changes |

## Pull Requests

1. **Target branch**: `development`
2. **Title**: Clear description of the change
3. **Body**: Reference the issue with `Closes #XX`
4. **Size**: Keep PRs focused and reviewable

```bash
# Example PR creation
gh pr create --base development --title "feat: add artwork cache" --body "Closes #42"
```

## Code Standards

### Go Backend

| Item | Convention |
|------|------------|
| Packages | lowercase, single word (`config`, `device`, `shortcuts`) |
| Exported | PascalCase (`DeviceClient`, `UploadGame`) |
| Unexported | camelCase (`parseConfig`, `buildPath`) |
| Comments | English |

### Svelte/TypeScript Frontend

| Item | Convention |
|------|------------|
| Components | PascalCase (`DeviceList.svelte`, `ArtworkGrid.svelte`) |
| Files | kebab-case for non-components (`api-client.ts`, `types.ts`) |
| Variables/Functions | camelCase |
| Types/Interfaces | PascalCase |
| CSS classes | Tailwind utilities |

### General Guidelines

- **Comments**: Write in English
- **Type hints**: Always use them (TypeScript)
- **Error handling**: Wrap errors with context in Go
- **Security**: Never log credentials or pairing tokens

## Building the Project

**Always use the build scripts. Never run build tools directly.**

```bash
# Linux/macOS — build all components
./build_all.sh

# Windows — build all components
build_all.bat

# Individual apps (from their directory)
cd apps/hub && ./build.sh
cd apps/agent && ./build.sh
```

## Labels

When creating issues, use appropriate labels:

| Category | Labels |
|----------|--------|
| Priority | `priority:critical`, `priority:high`, `priority:medium`, `priority:low` |
| Difficulty | `difficulty:easy`, `difficulty:medium`, `difficulty:hard` |
| Component | `hub`, `agent`, `decky-plugin`, `websocket`, `discovery`, `shortcuts`, `artwork` |

## Getting Help

- **Questions**: Open a [Discussion](https://github.com/lobinuxsoft/capydeploy/discussions)
- **Bugs**: Create an [Issue](https://github.com/lobinuxsoft/capydeploy/issues)

## License

By contributing, you agree that your contributions will be licensed under the AGPL v3.
