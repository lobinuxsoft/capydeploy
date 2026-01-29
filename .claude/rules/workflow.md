---
priority: critical
---

# WORKFLOW — DO NOT IGNORE

## Git & SemVer
- **Commits:** Spanish, Conventional (`feat:`, `fix:`, `docs:`). **NO signatures.**
- **Branches:** `main` (rel) <- `development` (int) <- `feat/issue-ID`.
- **SemVer:** MAJOR (Breaks, Tag `vX.0.0`), MINOR (Feat), PATCH (Fix).
- **Process:** PR `dev` -> `main`, Tag, Push. **NO Force Push.**

## GitHub CLI & Project
- **Flow:** Issue -> Branch -> PR -> **STOP** -> (User merges) -> Next issue.
- **Labels:** `priority:*`, `difficulty:*`, `next-session`.
- **Ops:**
  - `gh issue develop <NUM> --base master --checkout`
  - `gh pr create --base master --title "Title" --body "Closes #XX"`
  - `gh issue edit <NUM> --add-label next-session`

## PR Rules — CRITICAL
- **After PR creation:** STOP and wait for user instructions. **DO NOT** continue to next issue.
- **Merges:** User handles all PR merges. **NEVER merge** unless explicitly requested.
- **Next issue:** Only start when user gives the go-ahead.

## Build

### Hub (Wails App)
- **Location:** `apps/hub/`
- **Policy:** Use Wails CLI for builds. **NEVER manual go build.**
- **Commands:**
  - `cd apps/hub && wails dev` - Development mode with hot reload
  - `cd apps/hub && wails build` - Production build (current platform)
  - `cd apps/hub && wails generate module` - Regenerate frontend bindings after Go changes
- **Frontend:**
  - `cd apps/hub/frontend && bun install` - Install frontend deps
  - `cd apps/hub/frontend && bun run dev` - Frontend dev server (auto with wails dev)
- **Requirements:**
  - Go 1.23+
  - Bun (or Node.js 18+)
  - Windows: WebView2 (included in Win10+)
  - Linux: webkit2gtk-4.0
- **Cross-compile:** NOT supported. Build on target OS.

### Agent (Go daemon)
- **Location:** `apps/agent/`
- **Policy:** Use standard Go build. Cross-compile supported.
- **Commands:**
  - `cd apps/agent && go build -o capydeploy-agent .` - Build for current platform
  - `GOOS=linux GOARCH=amd64 go build -o capydeploy-agent .` - Linux build
  - `GOOS=windows GOARCH=amd64 go build -o capydeploy-agent.exe .` - Windows build
