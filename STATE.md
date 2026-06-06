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
- **P5 ✅** Packaged macOS app. Runtime rules path (`topics.rs` OnceLock + `lib.rs` `set_rules_dir` from the
  Tauri `resource_dir`; dev/test fallback) + review-queue runtime path (`learn.rs` `set_data_dir` → app data
  dir). `tauri.conf.json` identifier `cz.hapd.asmara`, productName Asmara, `bundle.resources` for rules, no
  app-sandbox. `npm run tauri build` → `Asmara.app` + dmg; launches clean, rules bundled at
  `Contents/Resources/resources/rules/`. **Live end-to-end verified** (Rust `live_ask_construction` ignored
  test): real `claude` → APPLIES §92a/§92e. check:all GREEN.
  - **Deployment follow-up (V5 auth):** the live ask works in dev / terminal-launched context (claude reads its
    keychain session). A Finder-launched `.app` may need `CLAUDE_CODE_OAUTH_TOKEN` (from `claude setup-token`)
    injected — `claude.rs` already uses it if present; plumbing a settings UI for it is a follow-up.
- **M3 ✅** Final review (correctness + security + simplify across the 5-commit diff): **no correctness or
  security bug** (fail-closed core + memory-poisoning gate intact; SQL parameterized; claude env clean). Applied:
  removed dead `LiveScorer.as_of_date`; corrected the false "persisted budget replaces per-process" comments.

### Follow-ups (from the M3 review — not blockers, deferred)
- **#1 (concurrency):** `submit_correction` holds the store `Mutex` across the multi-minute LLM gate (the gold
  sweep). Single-user-tolerable (one action at a time); for concurrency, split learn's API so verify runs
  off-lock. 
- **#2 (budget):** the persisted `budget` table is an unwired SEAM; the per-process `Budget` is the active
  guard. Wire cross-run enforcement when C2 metering bites.
- **#3:** narrow the crate-wide `#![allow(dead_code)]` once #2 is wired (currently masks the budget seam).
- **#5:** `solve.rs` parses atoms by string strip (fine for today's nullary args; reparse via `sym.name()`/
  `.arguments()` if a future rule emits compound args). **#7:** rename the crate off `tauri-starter` (careful —
  the lib name is build-load-bearing). **#6:** minor gate-render UI dup. **#8/#9:** cosmetic.

- **Next:** M4 — update `.github/workflows/ci.yml` for the clingo build (cmake + libclang + CMAKE policy), push
  to main of the tauri repo, CI green.
  P4 React UI (ask · correct · review · manage-knowledge) · P5 bundle + run · P6 check:all · then reviews + CI
  + push main.

## Build / verify
```
cd src-tauri && export CMAKE_POLICY_VERSION_MINIMUM=3.5 \
  && export LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/lib && cargo test
```
`.cargo/config.toml` sets the CMake policy; CI must `brew install cmake` + set `LIBCLANG_PATH`.
