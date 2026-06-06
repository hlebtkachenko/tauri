// engine — fail-closed neuro-symbolic core. P1: engine (route → extract → solve → gate).
// P2 (this phase): the datastore + continual-learning loop (store, gold, learn). No Tauri
// commands / frontend yet — that is P3.
//
// Flow on the request path: route → extract (LLM, enum-fenced) → solve (clingo) → gate.
// EVERY function on that path returns Result or maps errors to abstain; nothing guesses.
//
// The learning loop: episode → typed correction → distill → VERIFICATION GATE → provisional
// → human approve → trusted → retrieved into extraction. The gate is the product: a poisoned
// lesson that regresses the gold set is blocked, and approval is refused unless it passed.
//
// No Tauri commands wire the engine to the frontend yet (P3), so the public API is "unused"
// from the crate's perspective. Allow dead_code here until P3 consumes it; the unit tests
// exercise the pure core (solve, gate, extract parsing, topics, budget, store, learn).
#![allow(dead_code)]

pub mod budget;
pub mod classify;
pub mod claude;
pub mod extract;
pub mod gate;
pub mod gold;
pub mod learn;
pub mod route;
pub mod solve;
pub mod store;
pub mod topics;
pub mod types;
