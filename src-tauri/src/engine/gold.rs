// gold.rs — shared gold-set scoring for the learning verification gate. Ports gold.ts:
// `correct()` (status match; if solved, decision + citations-as-set match), `scoreGold`
// (run every gold case through the full pipeline), and the `Scorer` seam learn.ts injects.
//
// The `Scorer` trait is the seam: tests inject a deterministic MOCK; production uses
// `LiveScorer`, which runs the real pipeline (route → extract → solve → gate) with the
// candidate lesson's examples injected. The live path calls the `claude` CLI, so it is NOT
// unit-tested here; the trait keeps `verify()` in learn.rs synchronous and mockable.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::budget::Budget;
use super::extract::extract_for_topic;
use super::gate::gate;
use super::solve::solve;
use super::topics::load_topics;
use super::types::{EngineError, Outcome, StrategyItem, Topic, STATUS_SOLVED};

/// Gold-set accuracy + case count. The two numbers `verify()` compares baseline vs candidate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoldScore {
    pub accuracy: f64,
    pub total: usize,
}

/// The verification-gate scoring seam. `extra` is the candidate lesson(s) injected as
/// few-shot examples on top of trusted retrieval. Synchronous so `verify()` can be a plain
/// `fn` and tests can supply a deterministic mock with no LLM.
pub trait Scorer {
    fn score(&self, extra: &[StrategyItem]) -> Result<GoldScore, EngineError>;
}

/// A gold case's expected outcome (the `expect` block in cases.jsonl).
#[derive(Debug, Clone, serde::Deserialize)]
struct GoldExpect {
    status: String,
    #[serde(default)]
    decision: Option<String>,
    #[serde(default)]
    citations: Option<Vec<String>>,
}

/// One gold case. The natural-language question is `nl` (preferred) or `cz`.
#[derive(Debug, Clone, serde::Deserialize)]
struct GoldCase {
    #[serde(default)]
    nl: Option<String>,
    #[serde(default)]
    cz: Option<String>,
    expect: GoldExpect,
}

/// True if two citation lists are the same set (order-insensitive). Ports `sameSet`.
fn same_set(a: &[String], b: &[String]) -> bool {
    let x: BTreeSet<&String> = a.iter().collect();
    let y: BTreeSet<&String> = b.iter().collect();
    x == y
}

/// Did the pipeline outcome match the gold expectation? Ports `correct()`:
/// solved → answer with matching decision (and citations-as-set if specified); anything
/// else → abstain.
pub fn correct(
    out: &Outcome,
    expect_status: &str,
    decision: &Option<String>,
    citations: &Option<Vec<String>>,
) -> bool {
    if expect_status == STATUS_SOLVED {
        match out {
            Outcome::Answer {
                decision: got_decision,
                citations: got_citations,
                ..
            } => {
                if Some(got_decision) != decision.as_ref() {
                    return false;
                }
                if let Some(want) = citations {
                    if !same_set(got_citations, want) {
                        return false;
                    }
                }
                true
            }
            Outcome::Abstain { .. } => false,
        }
    } else {
        matches!(out, Outcome::Abstain { .. })
    }
}

/// The production scorer: run every topic's gold cases through the full pipeline with the
/// `extra` examples injected, like gold.ts `scoreGold(depsWithExamples(extra))`. Calls the
/// `claude` CLI (route + extract), so this is NOT unit-tested.
pub struct LiveScorer {
    /// As-of date for rule selection (the local path reads rules from the topic module).
    pub as_of_date: String,
}

impl LiveScorer {
    pub fn new(as_of_date: impl Into<String>) -> Self {
        LiveScorer {
            as_of_date: as_of_date.into(),
        }
    }

    /// Run a single gold case through extract → solve → gate (topic already chosen), with
    /// the candidate examples injected. Returns the outcome.
    async fn run_case(
        topic: &Topic,
        question: &str,
        extra: &[StrategyItem],
        budget: &mut Budget,
    ) -> Result<Outcome, EngineError> {
        // Inject the candidate lesson(s) as few-shot examples (title, content), mirroring
        // depsWithExamples — trusted retrieval would be added here in the full stack.
        let examples: Vec<(String, String)> = extra
            .iter()
            .map(|e| (e.title.clone(), e.content.clone()))
            .collect();
        let ruleset = fs::read_to_string(Path::new(&topic.rules_path))
            .map_err(|e| EngineError::Topics(format!("read {}: {e}", topic.rules_path)))?;
        // extract_for_topic already returns ASP facts; solve directly.
        let facts = extract_for_topic(question, topic, &examples, budget).await?;
        let result = solve(&ruleset, &facts);
        Ok(gate(&result))
    }

    /// Async core of scoring. The blocking `Scorer::score` drives this on a runtime.
    async fn score_async(&self, extra: &[StrategyItem]) -> Result<GoldScore, EngineError> {
        let topics = load_topics()?;
        let mut budget = Budget::from_env();
        let mut total = 0usize;
        let mut ok = 0usize;
        for topic in &topics {
            let path = Path::new(&topic.rules_path)
                .parent()
                .map(|p| p.join("gold").join("cases.jsonl"));
            let Some(path) = path else { continue };
            if !path.exists() {
                continue;
            }
            let text = fs::read_to_string(&path)
                .map_err(|e| EngineError::Topics(format!("read {}: {e}", path.display())))?;
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let case: GoldCase = serde_json::from_str(line)
                    .map_err(|e| EngineError::Topics(format!("parse gold case: {e}")))?;
                let Some(q) = case.nl.or(case.cz) else {
                    continue;
                };
                total += 1;
                // A failed run scores as wrong (fail-closed), never aborts the sweep.
                let out = Self::run_case(topic, &q, extra, &mut budget).await;
                if let Ok(out) = out {
                    if correct(
                        &out,
                        &case.expect.status,
                        &case.expect.decision,
                        &case.expect.citations,
                    ) {
                        ok += 1;
                    }
                }
            }
        }
        let accuracy = if total > 0 {
            ok as f64 / total as f64
        } else {
            0.0
        };
        Ok(GoldScore { accuracy, total })
    }
}

impl Scorer for LiveScorer {
    fn score(&self, extra: &[StrategyItem]) -> Result<GoldScore, EngineError> {
        // Drive the async pipeline on a current-thread runtime. The Store (!Send) is never
        // held here, so blocking on a runtime is safe.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| EngineError::Learn(format!("runtime build failed: {e}")))?;
        rt.block_on(self.score_async(extra))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_solved_matches_decision_and_citation_set() {
        let out = Outcome::Answer {
            decision: "reverse_charge_applies".to_string(),
            citations: vec!["§92e".to_string(), "§92a".to_string()],
            justification: vec![],
        };
        assert!(correct(
            &out,
            "solved",
            &Some("reverse_charge_applies".to_string()),
            &Some(vec!["§92a".to_string(), "§92e".to_string()]), // order-insensitive
        ));
        // wrong decision
        assert!(!correct(
            &out,
            "solved",
            &Some("reverse_charge_not_applies".to_string()),
            &None,
        ));
        // citation set mismatch
        assert!(!correct(
            &out,
            "solved",
            &Some("reverse_charge_applies".to_string()),
            &Some(vec!["§92a".to_string()]),
        ));
    }

    #[test]
    fn correct_expects_abstain_for_nonsolved() {
        let abstain = Outcome::Abstain {
            reason: "abstain".to_string(),
        };
        assert!(correct(&abstain, "abstain", &None, &None));
        let answer = Outcome::Answer {
            decision: "x".to_string(),
            citations: vec!["§1".to_string()],
            justification: vec![],
        };
        assert!(!correct(&answer, "abstain", &None, &None));
    }
}
