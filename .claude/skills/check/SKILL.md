---
description: Run the full verification sequence (format, lint, type-check, test) for both the TypeScript frontend and the Rust backend. Use before committing or when asked to check the code.
argument-hint: (no arguments)
---

Run the project's verification sequence and report results. Stop at the first failing step and show its output.

## Frontend (from project root)

```bash
npm run lint        # Biome: lint + format check + import order
npx tsc --noEmit    # type-check
```

## Rust (from src-tauri/)

```bash
cd src-tauri && cargo fmt --check && cargo clippy -- -D warnings && cargo check && cargo test
```

If `src-tauri/` or `package.json` does not exist yet, report that the project is still a stub and skip the missing half.
