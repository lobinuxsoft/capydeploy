---
name: pr
description: Create Pull Requests following project workflow.
---
# Pull Request Workflow

## Branch Structure
```
main (releases) <- development (integration) <- feature/issue-ID-desc
```

## Flow
1. **Issue First**: Siempre crear issue antes de trabajar
2. **Branch**: Crear desde issue con `gh issue develop`
3. **Work**: Commits en espanol, Conventional Commits
4. **PR**: Crear PR hacia `development`
5. **Close**: PR cierra el issue automaticamente

## GitHub CLI Commands

### Crear branch desde issue
```bash
gh issue develop <NUM> --base development --checkout
```

### Crear Pull Request
```bash
gh pr create --base development --title "feat: descripcion" --body "Closes #XX"
```

### Agregar labels a issue
```bash
gh issue edit <NUM> --add-label "priority:high"
gh issue edit <NUM> --add-label "next-session"
```

## Labels
- `priority:low|medium|high|critical`
- `difficulty:easy|medium|hard`
- `next-session` - Para retomar en proxima sesion
- `platform:windows|linux` - Especifico de plataforma
- `component:ui|device|config|shortcuts` - Area del codigo

## PR Template
```markdown
## Resumen
Breve descripcion de los cambios.

## Cambios
- Cambio 1
- Cambio 2

## Testing
- [ ] Compilado en Windows
- [ ] Compilado en Linux
- [ ] Tests pasan (`make test`)
- [ ] Probado manualmente con dispositivo

## Plataformas
- [ ] Windows
- [ ] Linux

Closes #XX
```

## SemVer (para releases main)
- **MAJOR** (vX.0.0): Breaking changes, incompatibilidad config
- **MINOR** (v0.X.0): Nueva funcionalidad
- **PATCH** (v0.0.X): Bug fixes

## Rules
- NUNCA force push a `main` o `development`
- PRs siempre van a `development`, no a `main`
- Releases: PR de `development` -> `main` + tag
- Probar en ambas plataformas antes de merge (cuando sea posible)
