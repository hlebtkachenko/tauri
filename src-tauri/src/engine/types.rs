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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SlotSpec {
    /// Allowed values (excludes "unknown").
    #[serde(rename = "enum")]
    pub enum_: Vec<String>,
    /// ASP fact with "{}" replaced by the value, e.g. "vat_status(supplier, {})".
    #[serde(rename = "factTemplate")]
    pub fact_template: String,
}

/// A single accounting topic (e.g. DPH reverse-charge), loaded from a rules module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum EngineError {
    /// A topic-module manifest or rules file could not be read or parsed.
    Topics(String),
    /// The LLM driver failed (spawn, timeout, non-zero exit, bad JSON, is_error).
    Claude(String),
    /// A budget ceiling (call count or USD spend) was exceeded.
    Budget(String),
    /// A datastore operation failed (rusqlite error, bad enum, missing row) or could not
    /// be serialized/deserialized.
    Store(String),
    /// A learning-loop precondition failed (orphan episode, missing governing §, etc.).
    Learn(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Topics(m) => write!(f, "topics error: {m}"),
            EngineError::Claude(m) => write!(f, "claude error: {m}"),
            EngineError::Budget(m) => write!(f, "budget error: {m}"),
            EngineError::Store(m) => write!(f, "store error: {m}"),
            EngineError::Learn(m) => write!(f, "learn error: {m}"),
        }
    }
}

impl std::error::Error for EngineError {}

// ---- Continual-learning loop (ADR-0007 + the four non-negotiables) -----------
// Ported from the TS reference `src/types.ts`. Every learning artefact carries
// provenance and a gate verdict; nothing self-generated is trusted until the
// verification gate passes (the gate is the product).

/// A token-space lesson distilled from a `fact_mapping` correction (ACE-style: kept as
/// data, delta-updated via helpful/harmful counters, never a weight update). Retrieved
/// k≈1 by tag and injected as a few-shot example into future extraction once TRUSTED.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct StrategyItem {
    pub title: String,
    pub description: String,
    pub content: String,
    pub tags: Vec<String>,
}

/// Staged trust. v1 has exactly two states (ADR-0006): a freshly distilled lesson is
/// `Provisional` and only a human approval (refused unless the gate passed) makes it
/// `Trusted` and therefore retrievable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TrustState {
    Provisional,
    Trusted,
}

impl TrustState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustState::Provisional => "provisional",
            TrustState::Trusted => "trusted",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, EngineError> {
        match s {
            "provisional" => Ok(TrustState::Provisional),
            "trusted" => Ok(TrustState::Trusted),
            other => Err(EngineError::Store(format!("invalid trust_state: {other}"))),
        }
    }
}

/// The correction type decides what gets learned (ADR-0007): only `FactMapping` becomes a
/// strategy item; `RuleDefect`/`VocabularyGap` edit the symbolic moat and route to a human.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionType {
    FactMapping,
    RuleDefect,
    VocabularyGap,
}

impl CorrectionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CorrectionType::FactMapping => "fact_mapping",
            CorrectionType::RuleDefect => "rule_defect",
            CorrectionType::VocabularyGap => "vocabulary_gap",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, EngineError> {
        match s {
            "fact_mapping" => Ok(CorrectionType::FactMapping),
            "rule_defect" => Ok(CorrectionType::RuleDefect),
            "vocabulary_gap" => Ok(CorrectionType::VocabularyGap),
            other => Err(EngineError::Store(format!(
                "invalid correction_type: {other}"
            ))),
        }
    }
}

/// Every solve, persisted as the raw material a correction attaches to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Episode {
    pub id: String,
    pub created_at: String,
    pub topic: Option<String>,
    pub question: String,
    pub as_of_date: String,
    pub facts: Option<Vec<String>>,
    pub decision: Option<String>,
    pub status: String,
    pub citations: Vec<String>,
}

/// Fields of an `Episode` minus the store-assigned `id` and `created_at`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct EpisodeInput {
    pub topic: Option<String>,
    pub question: String,
    pub as_of_date: String,
    pub facts: Option<Vec<String>>,
    pub decision: Option<String>,
    pub status: String,
    pub citations: Vec<String>,
}

/// An expert correction attached to an episode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Correction {
    pub id: String,
    pub created_at: String,
    pub episode_id: String,
    #[serde(rename = "type")]
    pub correction_type: CorrectionType,
    pub corrected_decision: Option<String>,
    /// The § the expert cites (required for a `fact_mapping` to pass citation-exists).
    pub governing_section: Option<String>,
    pub expert: String,
    pub note: Option<String>,
}

/// Fields of a `Correction` minus the store-assigned `id` and `created_at`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct CorrectionInput {
    pub episode_id: String,
    #[serde(rename = "type")]
    pub correction_type: CorrectionType,
    pub corrected_decision: Option<String>,
    pub governing_section: Option<String>,
    pub expert: String,
    pub note: Option<String>,
}

/// Where a strategy item came from. An item with no `episode_id`/`correction_id` is an
/// orphan (self-generated, unattributable) and the verification gate blocks it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, specta::Type)]
pub struct Provenance {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correction_id: Option<String>,
    pub source: String,
}

/// The verification-gate verdict. Approval is refused unless `passed` is true — a poisoned
/// lesson cannot be promoted even by a human.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct GateResult {
    pub passed: bool,
    /// Why it failed (empty when passed).
    pub reasons: Vec<String>,
    pub baseline_accuracy: f64,
    pub candidate_accuracy: f64,
}

/// A `StrategyItem` enriched with store/learning metadata: identity, staged trust,
/// provenance, delta counters, the gate verdict, and the human approval stamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct StoredStrategyItem {
    pub id: String,
    pub created_at: String,
    pub title: String,
    pub description: String,
    pub content: String,
    pub tags: Vec<String>,
    pub trust_state: TrustState,
    pub provenance: Provenance,
    // Delta counters (ACE-style helpful/harmful). `i32` so the bindings export to a plain
    // TS `number`; specta-typescript forbids `i64` (BigInt-style) to avoid precision loss.
    pub helpful: i32,
    pub harmful: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
}

impl StoredStrategyItem {
    /// The plain `StrategyItem` shape the extractor consumes (drops the metadata).
    pub fn to_strategy_item(&self) -> StrategyItem {
        StrategyItem {
            title: self.title.clone(),
            description: self.description.clone(),
            content: self.content.clone(),
            tags: self.tags.clone(),
        }
    }
}

/// Fields of a `StoredStrategyItem` minus the store-assigned `id` and `created_at`.
/// `add_strategy_item` takes this and stamps identity + timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct StrategyItemInput {
    pub title: String,
    pub description: String,
    pub content: String,
    pub tags: Vec<String>,
    pub trust_state: TrustState,
    pub provenance: Provenance,
    pub helpful: i32,
    pub harmful: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
}

/// Partial update for `update_strategy_item` (the promotion path). `None` fields are left
/// unchanged; identity (`id`, `created_at`) is never patched.
#[derive(Debug, Clone, Default)]
pub struct StrategyItemPatch {
    pub trust_state: Option<TrustState>,
    pub helpful: Option<i32>,
    pub harmful: Option<i32>,
    pub gate: Option<GateResult>,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
}
