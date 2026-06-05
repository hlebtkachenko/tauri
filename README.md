# tauri-starter

Personal Tauri 2.x scaffold. Copy it, re-stamp it, build on top.

**Stack:** Tauri 2.x (Rust) + React 19 + TypeScript 6 + Vite.

**Included:**
- Working Tauri app (demo `greet` IPC command) — `src/` (React), `src-tauri/` (Rust).
- Agent config: `AGENTS.md`, `CLAUDE.md`, `ARCHITECTURE.md`, `.claude/` (permissions, format hook, release-checker subagent, `/check` skill).
- Security & CI: gitleaks (pre-commit hook + GitHub Action), CI (type-check, build, `cargo fmt`/`clippy`/`check`/`test`), Dependabot, `SECURITY.md`.

## Start a new project from this scaffold

```bash
# 1. copy without this repo's git history / build output
cp -R tauri-starter my-app
cd my-app && rm -rf .git node_modules dist src-tauri/target src-tauri/Cargo.lock

# 2. re-stamp name + bundle id (+ optional display name)
./scripts/rename.sh my-app com.hleb.my-app "My App"

# 3. fresh git + enable the secret-scan hook
git init && git config core.hooksPath .githooks

# 4. install + run
npm install
npm run tauri dev
```

Requires Rust (`cargo`/`rustc`) and Node. See `AGENTS.md` for all commands.

## Resting scaffold identity (what `rename.sh` replaces)

| Field | Value |
|---|---|
| package / crate | `tauri-starter` / `tauri_starter_lib` |
| bundle id | `com.hleb.starter` |
| product / window title | `Tauri Starter` |
