# AGENTS.md

Shared instructions for AI coding agents (Claude Code, Cursor, Codex, etc.) working in this repository. This is the single source of truth; `CLAUDE.md` imports it.

## Project

A Tauri 2.x desktop application: Rust backend + React 19 / TypeScript 6 frontend (Vite), styled with Tailwind v4 + shadcn/ui.

## Structure

```
.
├── index.html              # frontend entry
├── package.json
├── vite.config.ts          # React + Tailwind v4 plugins, @/ alias
├── tsconfig.json
├── components.json         # shadcn/ui config
├── src/                    # React + TypeScript frontend
│   ├── main.tsx            # React entry (imports index.css)
│   ├── App.tsx
│   ├── index.css           # Tailwind v4 entry + shadcn theme (hex, not oklch)
│   ├── components/ui/      # shadcn/ui components (you own these)
│   ├── hooks/              # e.g. use-external-links.ts
│   └── lib/utils.ts        # cn() helper
└── src-tauri/              # Rust backend — all Tauri config lives here
    ├── Cargo.toml          # crate `tauri-starter`, lib `tauri_starter_lib`
    ├── build.rs            # tauri_build::build()
    ├── tauri.conf.json     # app id `com.hleb.starter`, devUrl, bundle
    ├── src/
    │   ├── lib.rs          # plugins + on_page_load (flash fix) + run()
    │   └── main.rs         # desktop entry — calls tauri_starter_lib::run()
    ├── capabilities/       # default.json + desktop.json (JS→Rust grants)
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
npm run check:all          # full gate: Biome + tsc + cargo fmt/clippy/check/test
npm run fix:all            # autofix: Biome + cargo fmt

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

## UI

- **Tailwind v4** (`@tailwindcss/vite`) + **shadcn/ui** (Radix-based). Theme lives in `src/index.css` as **hex** CSS variables (project rule — not oklch), light + `.dark`.
- Add components: `npx shadcn@latest add <name>` → lands in `src/components/ui/`, which you own and edit. Use `cn()` from `@/lib/utils`; icons via `lucide-react`.
- Import via the `@/` alias (e.g. `@/components/ui/button`).

## Tauri plugins

Wired: `opener`, `store`, `window-state`. Add more with `npm run tauri add <plugin>` (auto-edits Cargo.toml, lib.rs, capabilities). Plugins are deny-by-default — the matching `<plugin>:default` permission must be present in a capability file under `src-tauri/capabilities/`.

## Typed IPC (tauri-specta)

Rust commands are defined in `lib.rs` with `#[tauri::command] #[specta::specta]` and registered via `collect_commands!`. tauri-specta generates a fully-typed `src/lib/bindings.ts`; call commands from the frontend as `commands.<name>(...)` instead of stringly-typed `invoke`. Bindings regenerate on every `npm run tauri dev` and via `cargo test` (committed to the repo; excluded from Biome).

## Desktop batteries

- **No startup white flash**: window created hidden (`visible: false`), shown on page-load (`on_page_load` in `lib.rs`).
- **External links** open in the system browser (`useExternalLinks` hook + `plugin-opener`).
- **Desktop feel**: body scroll/overscroll locked and text selection disabled except in inputs (`index.css`).

## Releases

Tag `v*` (e.g. `npm version patch && git push --follow-tags`) → `.github/workflows/release.yml` builds installers for macOS (arm64 + x64), Windows, and Linux via `tauri-action` and opens a draft GitHub release. Release binaries use a size-optimized Cargo profile (`lto`, `strip`). Signed auto-updates are optional — see `SECURITY.md`.

## Scaffold notes

- **This is a reusable scaffold.** After copying for a new project, run `scripts/rename.sh <app-name> [bundle-id]` to re-stamp the package/crate/identifier from the resting identity (`tauri-starter` / `com.hleb.starter`). See `README.md`.
- **Rust required** — install `cargo`/`rustc` via `rustup` (https://www.rust-lang.org/tools/install) if not present.
- **Linter/formatter: Biome** (`biome.json`) — `npm run lint` / `npm run format`. Runs in CI + the PostToolUse hook. CSS is excluded (Tailwind v4 owns `src/index.css`).
- Frontend (tsc + vite) and Rust (`cargo check`) builds are verified in CI.

## Security

- Public repo. Secret scanning runs in CI (gitleaks) and locally via `.githooks/pre-commit` — enable once per clone: `git config core.hooksPath .githooks`.
- CI (`.github/workflows/ci.yml`) runs type-check, build, and `cargo fmt`/`clippy`/`check`/`test` on every push and PR. Dependabot keeps deps current. See `SECURITY.md`.

## Adding an MCP server

This repo ships no MCP servers. To add a team-shared one, create `.mcp.json` at the root (committed) with a `mcpServers` object; reference secrets via `${ENV_VAR}`, never hardcoded.
