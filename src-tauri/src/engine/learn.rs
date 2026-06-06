// learn.rs — the continual-learning loop (ADR-0007 + the four non-negotiables). Ported from
// learn.ts.
//
//   episode → typed correction
//     • FactMapping  → distill a token-space strategy item → VERIFICATION GATE
//                      (provenance present + cites governing § + re-run-vs-gold no-regression)
//                      → provisional → HUMAN approve → trusted → retrieved into extraction
//     • RuleDefect / VocabularyGap → queued for a human rule-edit (NOT autonomous)
//
// The gate is the product: a poisoned/contradicting lesson that regresses the gold set is
// blocked, and approval is refused unless the gate passed — even for a human approver.
//
// Fail-closed: the gate FAILS on a scorer error AND on an empty gold set (total == 0); an
// unverifiable lesson is never trusted. No unwrap/expect/panic on any path.

use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::gold::{GoldScore, Scorer};
use super::store::Store;
use super::topics::get_topic;
use super::types::{
    Correction, CorrectionInput, CorrectionType, EngineError, Episode, GateResult, Provenance,
    StoredStrategyItem, StrategyItem, StrategyItemInput, StrategyItemPatch, TrustState,
};

/// A distilled lesson before identity/timestamp/gate are stamped (the `addStrategyItem`
/// input minus the gate, mirroring the TS `Draft`).
#[derive(Debug, Clone)]
pub struct Draft {
    pub title: String,
    pub description: String,
    pub content: String,
    pub tags: Vec<String>,
    pub trust_state: TrustState,
    pub provenance: Provenance,
    pub helpful: i32,
    pub harmful: i32,
}

impl Draft {
    fn as_strategy_item(&self) -> StrategyItem {
        StrategyItem {
            title: self.title.clone(),
            description: self.description.clone(),
            content: self.content.clone(),
            tags: self.tags.clone(),
        }
    }

    fn into_input(self, gate: GateResult) -> StrategyItemInput {
        StrategyItemInput {
            title: self.title,
            description: self.description,
            content: self.content,
            tags: self.tags,
            trust_state: self.trust_state,
            provenance: self.provenance,
            helpful: self.helpful,
            harmful: self.harmful,
            gate: Some(gate),
            approved_by: None,
            approved_at: None,
        }
    }
}

/// Process-global data dir for human-review artifacts. Set once at app startup (the Tauri
/// app data dir); falls back to the crate `.context/` for cargo tests + `tauri dev`.
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Point the review queue at the runtime data directory (the Tauri app data dir). Called from
/// `lib.rs` `.setup(...)`. No-op if already set. Without it (dev/tests) the fallback is used.
pub fn set_data_dir(p: PathBuf) {
    let _ = DATA_DIR.set(p);
}

/// The review queue for non-`FactMapping` corrections: a human-readable markdown checklist.
/// Written under the runtime data dir in the packaged app (the crate `.context/` in dev/tests).
/// These edit the symbolic moat and are never applied autonomously.
fn review_queue_path() -> PathBuf {
    DATA_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(".context"))
        .join("review-queue.md")
}

/// Distill a `fact_mapping` correction into a provisional, fully-attributed lesson.
pub fn distill(episode: &Episode, correction: &Correction, tags: Vec<String>) -> Draft {
    let sec = correction
        .governing_section
        .clone()
        .unwrap_or_else(|| "rule".to_string());
    let corrected = correction
        .corrected_decision
        .clone()
        .unwrap_or_else(|| "correction".to_string());
    let note_suffix = correction
        .note
        .as_ref()
        .map(|n| format!(" {n}"))
        .unwrap_or_default();
    Draft {
        title: format!("{sec}: {corrected}"),
        description: correction
            .note
            .clone()
            .unwrap_or_else(|| "expert correction".to_string()),
        content: format!(
            "For a case like \"{}\", the correct outcome is {} ({}).{}",
            episode.question, corrected, sec, note_suffix
        ),
        tags,
        trust_state: TrustState::Provisional,
        provenance: Provenance {
            episode_id: Some(episode.id.clone()),
            correction_id: Some(correction.id.clone()),
            source: format!("expert:{}", correction.expert),
        },
        helpful: 0,
        harmful: 0,
    }
}

/// VERIFICATION GATE: provenance present + (implicitly) a cited governing § + no gold-set
/// regression. Fail-closed: a scorer error OR an empty gold set (`total == 0`) FAILS the
/// gate. Synchronous: the `Scorer` seam runs the (async) pipeline internally for the live
/// path; tests inject a deterministic mock.
pub fn verify(draft: &Draft, scorer: &dyn Scorer) -> GateResult {
    let mut reasons: Vec<String> = Vec::new();
    if draft.provenance.episode_id.is_none() || draft.provenance.correction_id.is_none() {
        reasons.push("missing provenance (orphan learning)".to_string());
    }
    if draft.provenance.source.is_empty() {
        reasons.push("missing source".to_string());
    }

    // Re-run-vs-gold. Any error here FAILS the gate (an unverifiable lesson is never
    // trusted); an empty gold set means there is nothing to verify against → reject.
    let baseline: GoldScore = match scorer.score(&[]) {
        Ok(s) => s,
        Err(e) => {
            return GateResult {
                passed: false,
                reasons: vec![format!("gate verification errored: {e}")],
                baseline_accuracy: 0.0,
                candidate_accuracy: 0.0,
            };
        }
    };
    let candidate: GoldScore = match scorer.score(&[draft.as_strategy_item()]) {
        Ok(s) => s,
        Err(e) => {
            return GateResult {
                passed: false,
                reasons: vec![format!("gate verification errored: {e}")],
                baseline_accuracy: 0.0,
                candidate_accuracy: 0.0,
            };
        }
    };

    if baseline.total == 0 {
        reasons.push("no gold cases to verify against".to_string());
    }
    if candidate.accuracy < baseline.accuracy {
        reasons.push(format!(
            "regresses gold set: {:.3} < {:.3}",
            candidate.accuracy, baseline.accuracy
        ));
    }
    GateResult {
        passed: reasons.is_empty(),
        reasons,
        baseline_accuracy: baseline.accuracy,
        candidate_accuracy: candidate.accuracy,
    }
}

/// Append a non-`FactMapping` correction to the human review queue (never autonomous).
fn queue_for_human(c: &Correction, kind: CorrectionType) -> Result<(), EngineError> {
    let path = review_queue_path();
    if let Some(dir) = path.parent() {
        create_dir_all(dir).map_err(|e| EngineError::Learn(format!("review queue mkdir: {e}")))?;
    }
    let line = format!(
        "- [ ] {} **{}** (episode {}) by {}: {} {} — {}\n",
        c.created_at,
        kind.as_str(),
        c.episode_id,
        c.expert,
        c.corrected_decision.as_deref().unwrap_or(""),
        c.governing_section.as_deref().unwrap_or(""),
        c.note.as_deref().unwrap_or(""),
    );
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| EngineError::Learn(format!("review queue open: {e}")))?;
    f.write_all(line.as_bytes())
        .map_err(|e| EngineError::Learn(format!("review queue write: {e}")))
}

/// Where a `submit_correction` was routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Routed {
    /// A `fact_mapping` lesson went through the verification gate.
    Gate,
    /// A rule/vocabulary correction was queued for a human (no strategy item created).
    Human,
}

/// The result of submitting a correction: where it routed, the persisted correction, and
/// (for the gate path) the created strategy item with its gate verdict.
#[derive(Debug, Clone)]
pub struct SubmitOutcome {
    pub routed: Routed,
    pub correction: Correction,
    pub item: Option<StoredStrategyItem>,
}

/// Submit an expert correction.
///   • `fact_mapping` (must cite a governing §) → distill → gate → store a PROVISIONAL item
///     carrying the gate verdict.
///   • `rule_defect` / `vocabulary_gap` → write to the human review queue; no item created.
pub fn submit_correction(
    store: &dyn Store,
    input: CorrectionInput,
    scorer: &dyn Scorer,
) -> Result<SubmitOutcome, EngineError> {
    let kind = input.correction_type;
    let correction = store.append_correction(input)?;

    // Non-fact_mapping corrections edit the symbolic moat → human rule-edit, never auto.
    if kind != CorrectionType::FactMapping {
        queue_for_human(&correction, kind)?;
        return Ok(SubmitOutcome {
            routed: Routed::Human,
            correction,
            item: None,
        });
    }

    let episode = store.get_episode(&correction.episode_id)?.ok_or_else(|| {
        EngineError::Learn(format!("episode {} not found", correction.episode_id))
    })?;

    // citation-exists: a fact_mapping lesson must cite the governing authority.
    if correction.governing_section.is_none() {
        return Err(EngineError::Learn(
            "fact_mapping correction must cite a governing § (citation-exists)".to_string(),
        ));
    }

    let tags = match &episode.topic {
        Some(topic_id) => get_topic(topic_id)?.map(|t| t.tags).unwrap_or_default(),
        None => Vec::new(),
    };
    let draft = distill(&episode, &correction, tags);
    let gate = verify(&draft, scorer);
    let item = store.add_strategy_item(draft.into_input(gate))?;
    Ok(SubmitOutcome {
        routed: Routed::Gate,
        correction,
        item: Some(item),
    })
}

/// HUMAN approval (provisional → trusted). Refused unless the gate passed — a poisoned
/// lesson cannot be promoted even by a human. Idempotent on an already-trusted item.
pub fn approve(
    store: &dyn Store,
    id: &str,
    approver: &str,
) -> Result<StoredStrategyItem, EngineError> {
    let item = store
        .list_strategy_items(None)?
        .into_iter()
        .find(|x| x.id == id)
        .ok_or_else(|| EngineError::Learn(format!("strategy item {id} not found")))?;

    if item.trust_state == TrustState::Trusted {
        return Ok(item);
    }

    let gate_passed = item.gate.as_ref().map(|g| g.passed).unwrap_or(false);
    if !gate_passed {
        let why = item
            .gate
            .as_ref()
            .map(|g| g.reasons.join("; "))
            .unwrap_or_else(|| "no gate run".to_string());
        return Err(EngineError::Learn(format!(
            "cannot approve: verification gate did not pass ({why})"
        )));
    }

    store
        .update_strategy_item(
            id,
            StrategyItemPatch {
                trust_state: Some(TrustState::Trusted),
                approved_by: Some(approver.to_string()),
                approved_at: Some(now_iso()),
                ..Default::default()
            },
        )?
        .ok_or_else(|| EngineError::Learn(format!("strategy item {id} vanished during approve")))
}

/// Best-effort ISO-ish UTC timestamp for the approval stamp, without a date dependency.
fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch:{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::store::SqliteStore;
    use crate::engine::types::EpisodeInput;

    // --- Deterministic mock scorers (NO LLM) ----------------------------------------
    // Baseline 0.8; a good lesson holds/raises it, a poisoned one drops it below baseline.
    struct GoodScorer;
    impl Scorer for GoodScorer {
        fn score(&self, extra: &[StrategyItem]) -> Result<GoldScore, EngineError> {
            Ok(GoldScore {
                accuracy: if extra.is_empty() { 0.8 } else { 0.9 },
                total: 5,
            })
        }
    }

    struct PoisonScorer;
    impl Scorer for PoisonScorer {
        fn score(&self, extra: &[StrategyItem]) -> Result<GoldScore, EngineError> {
            Ok(GoldScore {
                accuracy: if extra.is_empty() { 0.8 } else { 0.6 },
                total: 5,
            })
        }
    }

    struct ThrowingScorer;
    impl Scorer for ThrowingScorer {
        fn score(&self, _extra: &[StrategyItem]) -> Result<GoldScore, EngineError> {
            Err(EngineError::Learn("solver down".to_string()))
        }
    }

    struct EmptyGoldScorer;
    impl Scorer for EmptyGoldScorer {
        fn score(&self, _extra: &[StrategyItem]) -> Result<GoldScore, EngineError> {
            Ok(GoldScore {
                accuracy: 0.0,
                total: 0,
            })
        }
    }

    fn store() -> SqliteStore {
        SqliteStore::in_memory().expect("in-memory store")
    }

    fn seed_episode(s: &SqliteStore) -> Episode {
        s.append_episode(EpisodeInput {
            topic: Some("dph-reverse-charge".to_string()),
            question: "a case the model got wrong".to_string(),
            as_of_date: "2026-01-01".to_string(),
            facts: Some(vec![]),
            decision: None,
            status: "abstain".to_string(),
            citations: vec![],
        })
        .expect("seed episode")
    }

    fn fm(episode_id: &str, expert: &str) -> CorrectionInput {
        CorrectionInput {
            episode_id: episode_id.to_string(),
            correction_type: CorrectionType::FactMapping,
            corrected_decision: Some("reverse_charge_applies".to_string()),
            governing_section: Some("§92e".to_string()),
            expert: expert.to_string(),
            note: None,
        }
    }

    // good fact_mapping lesson passes the gate but stays provisional (no auto-trust).
    #[test]
    fn good_lesson_passes_gate_stays_provisional() {
        let s = store();
        let ep = seed_episode(&s);
        let out = submit_correction(&s, fm(&ep.id, "hleb"), &GoodScorer).expect("submit");
        assert_eq!(out.routed, Routed::Gate);
        let item = out.item.expect("item");
        assert!(item.gate.as_ref().expect("gate").passed);
        assert_eq!(item.trust_state, TrustState::Provisional);
        assert_eq!(item.provenance.episode_id.as_deref(), Some(ep.id.as_str()));
    }

    // poisoned lesson that regresses the gold set is BLOCKED by the gate, and a human
    // cannot approve it.
    #[test]
    fn poison_lesson_blocked_and_unapprovable() {
        let s = store();
        let ep = seed_episode(&s);
        let out = submit_correction(&s, fm(&ep.id, "evil"), &PoisonScorer).expect("submit");
        let item = out.item.expect("item");
        let gate = item.gate.as_ref().expect("gate");
        assert!(!gate.passed);
        assert!(gate.reasons.join(" ").contains("regress"));
        // even a human cannot approve a gate-failed lesson
        let err = approve(&s, &item.id, "hleb").expect_err("must refuse");
        assert!(err.to_string().contains("gate did not pass"));
    }

    // human approval promotes a gate-passed item provisional → trusted (then retrievable).
    #[test]
    fn approve_promotes_gate_passed_item() {
        let s = store();
        let ep = seed_episode(&s);
        let out = submit_correction(&s, fm(&ep.id, "hleb"), &GoodScorer).expect("submit");
        let item = out.item.expect("item");
        // provisional → not retrieved
        assert_eq!(
            s.trusted_by_tags(&["dph".to_string()], &[], 1)
                .expect("retrieve")
                .len(),
            0
        );
        let trusted = approve(&s, &item.id, "hleb").expect("approve");
        assert_eq!(trusted.trust_state, TrustState::Trusted);
        assert_eq!(trusted.approved_by.as_deref(), Some("hleb"));
        // now injected into extraction
        assert_eq!(
            s.trusted_by_tags(&["dph".to_string()], &[], 1)
                .expect("retrieve")
                .len(),
            1
        );
    }

    // fact_mapping without a governing § is rejected (citation-exists).
    #[test]
    fn fact_mapping_without_section_rejected() {
        let s = store();
        let ep = seed_episode(&s);
        let input = CorrectionInput {
            episode_id: ep.id.clone(),
            correction_type: CorrectionType::FactMapping,
            corrected_decision: Some("x".to_string()),
            governing_section: None,
            expert: "hleb".to_string(),
            note: None,
        };
        let err = submit_correction(&s, input, &GoodScorer).expect_err("must reject");
        assert!(err.to_string().contains("governing"));
    }

    // rule_defect is routed to a human, never learned autonomously (no item created).
    #[test]
    fn rule_defect_routed_to_human() {
        let s = store();
        let ep = seed_episode(&s);
        let input = CorrectionInput {
            episode_id: ep.id.clone(),
            correction_type: CorrectionType::RuleDefect,
            corrected_decision: None,
            governing_section: Some("§92e".to_string()),
            expert: "hleb".to_string(),
            note: Some("rule incomplete".to_string()),
        };
        let out = submit_correction(&s, input, &GoodScorer).expect("submit");
        assert_eq!(out.routed, Routed::Human);
        assert!(out.item.is_none());
        assert_eq!(s.list_strategy_items(None).expect("list").len(), 0);
    }

    // gate fails closed when the gold re-run errors.
    #[test]
    fn gate_fails_on_scorer_error() {
        let s = store();
        let ep = seed_episode(&s);
        let out = submit_correction(&s, fm(&ep.id, "hleb"), &ThrowingScorer).expect("submit");
        let gate = out.item.expect("item").gate.expect("gate");
        assert!(!gate.passed);
        assert!(gate.reasons.join(" ").contains("errored"));
    }

    // gate fails when there are no gold cases to verify against (total == 0).
    #[test]
    fn gate_fails_on_empty_gold() {
        let s = store();
        let ep = seed_episode(&s);
        let out = submit_correction(&s, fm(&ep.id, "hleb"), &EmptyGoldScorer).expect("submit");
        let gate = out.item.expect("item").gate.expect("gate");
        assert!(!gate.passed);
        assert!(gate.reasons.join(" ").contains("no gold cases"));
    }

    // verify() blocks an orphan lesson (missing provenance).
    #[test]
    fn verify_blocks_orphan() {
        let draft = Draft {
            title: "t".to_string(),
            description: "d".to_string(),
            content: "c".to_string(),
            tags: vec!["dph".to_string()],
            trust_state: TrustState::Provisional,
            provenance: Provenance {
                episode_id: None,
                correction_id: None,
                source: "x".to_string(),
            },
            helpful: 0,
            harmful: 0,
        };
        let gate = verify(&draft, &GoodScorer);
        assert!(!gate.passed);
        assert!(gate.reasons.join(" ").contains("provenance"));
    }

    // approve refuses an item whose gate failed (defense in depth beyond the poison test).
    #[test]
    fn approve_refuses_gate_failed_item() {
        let s = store();
        let ep = seed_episode(&s);
        let out = submit_correction(&s, fm(&ep.id, "hleb"), &PoisonScorer).expect("submit");
        let item = out.item.expect("item");
        assert!(approve(&s, &item.id, "hleb").is_err());
        // and it is still provisional / not retrievable
        assert_eq!(
            s.trusted_by_tags(&["dph".to_string()], &[], 1)
                .expect("retrieve")
                .len(),
            0
        );
    }
}
