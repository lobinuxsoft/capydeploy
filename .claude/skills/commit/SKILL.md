---
name: commit
description: Create commits following project conventions.
---
# Git Commit

## Format
- **Language**: Spanish (message body)
- **Style**: Conventional Commits
- **NO AI signatures** (no "Co-Authored-By: Claude")

## Types
```
feat:     Nueva funcionalidad
fix:      Correccion de bug
docs:     Documentacion
style:    Formato (sin cambio de codigo)
refactor: Refactorizacion
perf:     Mejora de rendimiento
test:     Tests
chore:    Mantenimiento
build:    Cambios en build/deps
```

## Structure
```
<type>(<scope>): <descripcion corta>

[cuerpo opcional - que y por que]

[footer opcional - referencias a issues]
```

## Scopes (this project)
```
frontend:  UI Svelte/Tailwind
backend:   Go backend logic
device:    Cliente SSH/SFTP
config:    Configuracion
shortcuts: Shortcuts de Steam
artwork:   SteamGridDB integration
wails:     Wails bindings/config
build:     Build configuration
```

## Examples
```bash
feat(ui): agregar indicador de estado de conexion
fix(device): corregir timeout en escaneo de red
docs(readme): actualizar instrucciones de compilacion
refactor(config): extraer validacion de rutas SSH
perf(artwork): implementar cache de imagenes
```

## Process
1. `git status` - Verificar cambios
2. `git diff --staged` - Revisar lo que se commitea
3. Stage archivos especificos (evitar `git add .`)
4. Commit con mensaje descriptivo

## Rules
- Un commit = un cambio logico
- Mensaje describe el **que** y **por que**, no el **como**
- Si el commit necesita "y" en el mensaje, probablemente deberia ser 2 commits
