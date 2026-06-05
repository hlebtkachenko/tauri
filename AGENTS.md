# AGENTS.md

Shared instructions for AI coding agents (Claude Code, Cursor, Codex, etc.) working in this repository. This is the single source of truth; `CLAUDE.md` imports it.

## Project

A Tauri 2.x desktop application: Rust backend + React 19 / TypeScript 6 frontend (Vite).

## Structure

```
.
├── index.html              # frontend entry
├── package.json
├── vite.config.ts
├── tsconfig.json
├── src/                    # React + TypeScript frontend
│   ├── main.tsx            # React entry
│   ├── App.tsx
│   └── assets/
└── src-tauri/              # Rust backend — all Tauri config lives here
    ├── Cargo.toml          # crate `tauri-starter`, lib `tauri_starter_lib`
    ├── build.rs            # tauri_build::build()
    ├── tauri.conf.json     # app id `com.hleb.starter`, devUrl, bundle
    ├── src/
    │   ├── lib.rs          # app code + #[tauri::command]s + run()
    │   └── main.rs         # desktop entry — calls tauri_starter_lib::run()
    ├── capabilities/       # permission grants for JS→Rust commands
    └── icons/
```

`src-tauri/target/` and `dist/` are build output: never commit them. Commit `src-tauri/Cargo.lock`.

## Commands

```bash
# Development
npm run tauri dev          # frontend dev server + Tauri window (hot-reload)
npm run tauri build        # production bundle

# Frontend (project root)
npm run dev                # Vite dev server only (no Tauri window)
npm run build              # tsc type-check + vite build
npx tsc --noEmit           # type-check only
npm run lint               # Biome: lint + format check + import order
npm run format             # Biome: autofix (format + safe lint fixes)

# Rust (from src-tauri/) — requires the Rust toolchain
cargo check
cargo clippy -- -D warnings
cargo fmt --check          # cargo fmt to auto-fix
cargo test
```

**Self-check before committing Rust changes:**

```bash
cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings && cargo check && cargo test
```

## Conventions

- English only in all files, code, and comments.
- Conventional Commits: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`.
- TypeScript 6.0+ (installed: 6.0.x).
- Edit `src-tauri/src/lib.rs` for Rust logic; `main.rs` is a thin desktop shim.
- A Rust command must be granted in `src-tauri/capabilities/` before the frontend can call it via `invoke()`.
- `tauri.conf.json` lives in `src-tauri/`, not the project root.
- Never commit secrets (`.env`, `*.key`, `*.enc`, `client_secret*.json`). Agents are denied read/edit access in `.claude/settings.json`.

## Scaffold notes

- **This is a reusable scaffold.** After copying for a new project, run `scripts/rename.sh <app-name> [bundle-id]` to re-stamp the package/crate/identifier from the resting identity (`tauri-starter` / `com.hleb.starter`). See `README.md`.
- **Rust required** — install `cargo`/`rustc` via `rustup` (https://www.rust-lang.org/tools/install) if not present.
- **Linter/formatter: Biome** (`biome.json`) — `npm run lint` (check) / `npm run format` (autofix). Runs in CI and via the PostToolUse hook.
- Frontend (tsc + vite) and Rust (`cargo check`) builds are verified in CI.

## Security

- Public repo. Secret scanning runs in CI (gitleaks) and locally via `.githooks/pre-commit` — enable once per clone: `git config core.hooksPath .githooks`.
- CI (`.github/workflows/ci.yml`) runs type-check, build, and `cargo fmt`/`clippy`/`check`/`test` on every push and PR. Dependabot keeps deps current. See `SECURITY.md`.

## Adding an MCP server

This repo ships no MCP servers. To add a team-shared one, create `.mcp.json` at the root (committed) with a `mcpServers` object; reference secrets via `${ENV_VAR}`, never hardcoded.
