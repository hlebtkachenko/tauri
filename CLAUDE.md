# CLAUDE.md

@AGENTS.md

Claude Code reads this file; the shared project instructions live in `AGENTS.md` (imported above). The notes below are Claude-specific.

## Enforcement vs. guidance

`AGENTS.md` is guidance (advisory context). Hard rules are enforced in `.claude/settings.json`:

- **Permissions**: `src/`, `src-tauri/`, and config files are editable; reading or editing secrets (`.env`, `*.key`, `*.enc`, `client_secret*.json`) is denied, as are `git push`, `rm -rf`, and `sudo`. Deny rules always win over allow.
- **Hook**: a `PostToolUse` hook (`.claude/hooks/format.sh`) runs `prettier`/`rustfmt` on edited files. It is a safe no-op until the toolchain is installed.

## .claude/ layout

- `settings.json` — committed permissions, env, and the formatter hook.
- `settings.local.json` — gitignored personal overrides (create as needed).
- `agents/tauri-release-checker.md` — read-only pre-flight agent for releases.
- `skills/check/` — `/check` runs the full format/lint/type-check/test sequence.
- `hooks/format.sh` — formatter invoked by the PostToolUse hook.

## Conductor workspace

This checkout is a Conductor workspace (a git worktree of `repos/tauri`). Target branch is `origin/main`: diff with `git diff origin/main...`, open PRs with `gh pr create --base main`. `.context/` is gitignored scratch space for collaborating with other agents.

## Status

Scaffolded: Tauri 2.x + React 19 + TypeScript 6 + Vite. The Rust toolchain (`cargo`/`rustc`) is **not yet installed** — required for `tauri dev`/`build`; install via `rustup`. No linter is configured yet. The frontend type-checks and builds under TS 6.
