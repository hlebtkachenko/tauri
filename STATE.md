# STATE — research-power → Tauri migration

Migrating the research-power engine (fail-closed neuro-symbolic Czech-**accounting** expert + continual-learning
loop) INTO this Tauri 2 app. Durable loop + research:
`research-power/chiang-mai/.context/MIGRATION-LOOP.md` + `MIGRATION-RESEARCH.md`. Reference impl:
research-power `master` `src/` + `solver/` + `docs/tauri-handoff.md`.

## Progress
- **P0 ✅** clingo-rs `static-linking` spike GREEN — native ASP in-binary (no Python sidecar), output matches
  Python clingo 5.8 exactly. Build needs `cmake` + env `CMAKE_POLICY_VERSION_MINIMUM=3.5` + `LIBCLANG_PATH`.
- **P1 ✅** engine core in `src-tauri/src/engine/` (types, topics, solve [clingo-rs], gate, budget, extract,
  route, claude). `cargo test` 26/26; clippy `-D warnings` + fmt clean; **solve-vs-gold == Python baseline**
  both topics. Advisor-gated: semantically faithful + fail-closed (no panic on the hot path).
- **P2 ✅** SQLite store (rusqlite bundled) + continual-learning loop (`store.rs`, `gold.rs`, `learn.rs`).
  `cargo test` 42/42; clippy + fmt clean. Advisor security review (memory-poisoning): **no path to
  trusted+injected** for a blocked lesson; gate fail-closed on scorer-error/empty-gold/regression/missing
  provenance+§; provisional never retrieved.
- **P3 ✅** Tauri commands + typed bindings + progress Channel. `engine/classify.rs` (the `ask` pipeline,
  fail-closed → always an Outcome, store lock never across `.await`) + commands `ask`/`submit_correction`/
  `approve`/`list_topics`/`recent_episodes`/`list_strategy_items` (tauri-specta, regenerated `src/lib/bindings.ts`).
  `npm run check:all` GREEN (42/42 + biome + tsc + clippy + fmt). Advisor-gated fail-closed.
- **P4 ✅** React UI — 4 shadcn surfaces (`src/components/asmara/{ask,correct,review,manage}-tab.tsx` + tabbed
  `App.tsx`): Ask (live stage progress via Channel → answer/abstain card), Correct (→ gate verdict), Review
  (approve disabled unless `gate.passed`), Manage (topics + episodes). `check:all` GREEN + `vite build` OK.
  Advisor-gated: ask never shows a raw error (fail-closed to abstain); async handlers try/finally; approve gated.
- **Next:** P5 bundle + run ·
  P4 React UI (ask · correct · review · manage-knowledge) · P5 bundle + run · P6 check:all · then reviews + CI
  + push main.

## Build / verify
```
cd src-tauri && export CMAKE_POLICY_VERSION_MINIMUM=3.5 \
  && export LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/lib && cargo test
```
`.cargo/config.toml` sets the CMake policy; CI must `brew install cmake` + set `LIBCLANG_PATH`.
