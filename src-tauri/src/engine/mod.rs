// engine — fail-closed neuro-symbolic core (P1: engine only; no datastore, no learning
// loop, no Tauri commands yet).
//
// Flow on the request path: route → extract (LLM, enum-fenced) → solve (clingo) → gate.
// EVERY function on that path returns Result or maps errors to abstain; nothing guesses.
//
// P1 has no Tauri commands wiring the engine to the frontend yet, so the public API is
// "unused" from the crate's perspective. Allow dead_code here until P2/P3 consume it;
// the unit tests exercise the pure core (solve, gate, extract parsing, topics, budget).
#![allow(dead_code)]

pub mod budget;
pub mod claude;
pub mod extract;
pub mod gate;
pub mod route;
pub mod solve;
pub mod topics;
pub mod types;
