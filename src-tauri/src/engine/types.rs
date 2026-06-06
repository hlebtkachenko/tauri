// Shared engine types. Mirrors the reference `src/types.ts` and the status values
// emitted by `solver/solve.py`. Fail-closed by construction: an `Outcome` is either a
// fully-cited `Answer` or an `Abstain` with a reason — never a bare guess.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Solver status. Drives compile-or-refuse in the gate.
/// solved = a determinate decision; abstain = insufficient facts; conflict =
/// contradictory input; schema_invalid = facts/rules failed to parse.
pub const STATUS_SOLVED: &str = "solved";
pub const STATUS_ABSTAIN: &str = "abstain";
pub const STATUS_CONFLICT: &str = "conflict";
pub const STATUS_SCHEMA_INVALID: &str = "schema_invalid";

/// The regulated answer, or a refusal. The single thing the gate produces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    Answer {
        decision: String,
        citations: Vec<String>,
        justification: Vec<String>,
    },
    Abstain {
        reason: String,
    },
}

/// One slot in a topic's FIXED vocabulary. Lives in `manifest.json` (data, not code),
/// so a new accounting area is a drop-in module — never an engine edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotSpec {
    /// Allowed values (excludes "unknown").
    #[serde(rename = "enum")]
    pub enum_: Vec<String>,
    /// ASP fact with "{}" replaced by the value, e.g. "vat_status(supplier, {})".
    #[serde(rename = "factTemplate")]
    pub fact_template: String,
}

/// A single accounting topic (e.g. DPH reverse-charge), loaded from a rules module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Topic {
    /// Module id, e.g. "dph-reverse-charge".
    pub id: String,
    /// rule_version.ruleset key, e.g. "dph_reverse_charge".
    pub ruleset: String,
    /// One line, used by the router.
    pub description: String,
    /// Extraction system prompt (reason-then-emit).
    #[serde(rename = "systemPrompt")]
    pub system_prompt: String,
    pub slots: BTreeMap<String, SlotSpec>,
    pub tags: Vec<String>,
    /// Absolute path to rules.lp (resolved at load time; not part of the manifest).
    #[serde(default, rename = "rulesPath")]
    pub rules_path: String,
}

/// Topic-agnostic result of one ASP solve. Mirrors `SolverResult` in the TS reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SolverResult {
    pub decision: Option<String>,
    pub status: String,
    pub citations: Vec<String>,
    pub justification: Vec<String>,
}

/// Engine error. Every fallible boundary maps to one of these or to an abstain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineError {
    /// A topic-module manifest or rules file could not be read or parsed.
    Topics(String),
    /// The LLM driver failed (spawn, timeout, non-zero exit, bad JSON, is_error).
    Claude(String),
    /// A budget ceiling (call count or USD spend) was exceeded.
    Budget(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Topics(m) => write!(f, "topics error: {m}"),
            EngineError::Claude(m) => write!(f, "claude error: {m}"),
            EngineError::Budget(m) => write!(f, "budget error: {m}"),
        }
    }
}

impl std::error::Error for EngineError {}
