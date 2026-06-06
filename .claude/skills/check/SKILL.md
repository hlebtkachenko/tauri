---
description: Run the full verification gate (Biome, type-check, cargo fmt/clippy/check/test) for the whole project. Use before committing or when asked to check the code.
argument-hint: (no arguments)
---

Run the project's full verification gate and report results. Stop at the first failing step and show its output.

```bash
npm run check:all
```

This runs, in order: Biome (`biome check`), `tsc --noEmit`, then `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo check`, and `cargo test`. To autofix formatting and safe lints: `npm run fix:all`.

If `src-tauri/` or `package.json` does not exist yet, report that the project is still a stub and skip the missing half.
