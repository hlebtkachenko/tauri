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
- **Next:** P2 store (rusqlite) + learn loop + gold scorer · P3 Tauri commands + specta bindings + Channel ·
  P4 React UI (ask · correct · review · manage-knowledge) · P5 bundle + run · P6 check:all · then reviews + CI
  + push main.

## Build / verify
```
cd src-tauri && export CMAKE_POLICY_VERSION_MINIMUM=3.5 \
  && export LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/lib && cargo test
```
`.cargo/config.toml` sets the CMake policy; CI must `brew install cmake` + set `LIBCLANG_PATH`.
