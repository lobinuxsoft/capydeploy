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
- **Flow:** Issue -> Branch -> PR -> Close.
- **Labels:** `priority:*`, `difficulty:*`, `next-session`.
- **Ops:**
  - `gh issue develop <NUM> --base development --checkout`
  - `gh pr create --base development --title "Title" --body "Closes #XX"`
  - `gh issue edit <NUM> --add-label next-session`

## Build (Wails)
- **Policy:** Use Wails CLI for builds. **NEVER manual go build.**
- **Commands:**
  - `wails dev` - Development mode with hot reload
  - `wails build` - Production build (current platform)
  - `wails build -platform windows/amd64` - Windows build
  - `wails build -platform linux/amd64` - Linux build
  - `wails generate module` - Regenerate frontend bindings after Go changes
- **Frontend:**
  - `cd frontend && npm install` - Install frontend deps
  - `cd frontend && npm run dev` - Frontend dev server (auto with wails dev)
- **Requirements:**
  - Go 1.23+
  - Node.js 18+
  - Windows: WebView2 (included in Win10+)
  - Linux: webkit2gtk-4.0
- **Cross-compile:** NOT supported. Build on target OS.
