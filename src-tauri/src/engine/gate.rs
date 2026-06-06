// gate.rs — compile-or-refuse / fail-closed. The single chokepoint for the regulated
// answer. Pure: given a solver result (or an infra failure), decide whether we have an
// answer or must abstain + escalate. Never guesses.

use super::types::{Outcome, SolverResult, STATUS_SOLVED};

/// Map a solver result to an outcome. Anything but a clean `solved` (with a decision AND
/// at least one citation) → abstain.
pub fn gate(result: &SolverResult) -> Outcome {
    if result.status == STATUS_SOLVED && result.decision.is_some() && !result.citations.is_empty() {
        Outcome::Answer {
            decision: result.decision.clone().unwrap_or_default(),
            citations: result.citations.clone(),
            justification: result.justification.clone(),
        }
    } else {
        Outcome::Abstain {
            reason: result.status.clone(),
        }
    }
}

/// Fail-closed: any boundary failure (solver down, LLM error, bad extraction) → abstain,
/// never guess.
pub fn gate_failure(reason: &str) -> Outcome {
    Outcome::Abstain {
        reason: format!("infra_failure:{reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::{STATUS_ABSTAIN, STATUS_CONFLICT, STATUS_SCHEMA_INVALID};

    fn solved() -> SolverResult {
        SolverResult {
            decision: Some("reverse_charge_applies".to_string()),
            status: STATUS_SOLVED.to_string(),
            citations: vec!["§92a".to_string(), "§92e".to_string()],
            justification: vec!["payers_both_vat".to_string()],
        }
    }

    #[test]
    fn solved_with_decision_and_citations_is_answer() {
        match gate(&solved()) {
            Outcome::Answer {
                decision,
                citations,
                justification,
            } => {
                assert_eq!(decision, "reverse_charge_applies");
                assert_eq!(citations, vec!["§92a", "§92e"]);
                assert_eq!(justification, vec!["payers_both_vat"]);
            }
            Outcome::Abstain { reason } => panic!("expected answer, got abstain: {reason}"),
        }
    }

    #[test]
    fn solved_but_no_citations_is_abstain() {
        let mut r = solved();
        r.citations.clear();
        assert_eq!(
            gate(&r),
            Outcome::Abstain {
                reason: STATUS_SOLVED.to_string()
            }
        );
    }

    #[test]
    fn solved_but_no_decision_is_abstain() {
        let mut r = solved();
        r.decision = None;
        assert!(matches!(gate(&r), Outcome::Abstain { .. }));
    }

    #[test]
    fn abstain_status_is_abstain() {
        let r = SolverResult {
            status: STATUS_ABSTAIN.to_string(),
            ..Default::default()
        };
        assert_eq!(
            gate(&r),
            Outcome::Abstain {
                reason: STATUS_ABSTAIN.to_string()
            }
        );
    }

    #[test]
    fn conflict_status_is_abstain() {
        let r = SolverResult {
            status: STATUS_CONFLICT.to_string(),
            ..Default::default()
        };
        assert!(matches!(gate(&r), Outcome::Abstain { .. }));
    }

    #[test]
    fn schema_invalid_status_is_abstain() {
        let r = SolverResult {
            status: STATUS_SCHEMA_INVALID.to_string(),
            ..Default::default()
        };
        assert!(matches!(gate(&r), Outcome::Abstain { .. }));
    }

    #[test]
    fn gate_failure_prefixes_reason() {
        assert_eq!(
            gate_failure("solver_down"),
            Outcome::Abstain {
                reason: "infra_failure:solver_down".to_string()
            }
        );
    }
}
