# tauri-starter

<!-- starter:remove-start -->
Personal Tauri 2.x scaffold. Copy it, re-stamp it, build on top.
<!-- starter:remove-end -->

**Stack:** Tauri 2.x (Rust) + React 19 + TypeScript 6 + Vite, Tailwind v4 + shadcn/ui.

**Included:**
- Tauri app with a `greet` typed-IPC example — `src/` (React), `src-tauri/` (Rust).
- Agent config: `AGENTS.md`, `CLAUDE.md`, `ARCHITECTURE.md`, `.claude/` (permissions, format hook, release-checker subagent, `/check` skill).
- Quality & security: Biome, gitleaks (pre-commit hook + CI), CI (typecheck/build/clippy/test), Dependabot, upstream-watch, release pipeline, `SECURITY.md`.

<!-- starter:remove-start -->
## Start a new project from this scaffold

```bash
# 1. copy without git history / build output
cp -R tauri-starter my-app
cd my-app && rm -rf .git node_modules dist src-tauri/target src-tauri/Cargo.lock

# 2. re-stamp name + bundle id (+ optional display name) across code AND docs
./scripts/rename.sh my-app com.hleb.my-app "My App"

# 3. fresh git + enable the secret-scan hook
git init && git config core.hooksPath .githooks

# 4. install + run
npm install
npm run tauri dev
```

`rename.sh` rewrites the package/crate/bundle-id/product name everywhere and strips these scaffold-only sections. Requires Rust (`cargo`/`rustc`) and Node.

## Resting scaffold identity (what `rename.sh` replaces)

| Field | Value |
|---|---|
| package / crate | `tauri-starter` / `tauri_starter_lib` |
| bundle id | `com.hleb.starter` |
| product / window title | `Tauri Starter` |
<!-- starter:remove-end -->

## Develop

```bash
npm install
npm run tauri dev      # dev window (hot-reload)
npm run check:all      # Biome + tsc + cargo fmt/clippy/check/test
npm run tauri build    # production bundle
```

See `AGENTS.md` for full conventions and commands.
