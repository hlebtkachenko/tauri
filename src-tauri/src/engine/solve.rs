// solve.rs — the ASP reasoning core (compile-or-refuse). Ports `solver/solve.py` onto
// clingo-rs (API proven by clingo-spike). Topic-agnostic: decision labels, citations and
// justification are emitted BY the rules as atoms, never hardcoded here.
//
// Fail-closed: never panics on bad input. Any clingo error (parse/ground/solve) maps to
// status = schema_invalid (refuse), exactly like the Python `except RuntimeError`.

use clingo::{control, Part, ShowType, SolveMode};

use super::types::{
    SolverResult, STATUS_ABSTAIN, STATUS_CONFLICT, STATUS_SCHEMA_INVALID, STATUS_SOLVED,
};

/// Normalize a fact line: trim and ensure exactly one trailing ".". Empty → empty.
fn normalize(fact: &str) -> String {
    let f = fact.trim();
    if f.is_empty() {
        return String::new();
    }
    format!("{}.", f.trim_end_matches('.'))
}

/// The atoms we care about across all models: emitted decisions, citations, justification.
#[derive(Default)]
struct ShownAtoms {
    decisions: Vec<String>,
    cites: Vec<String>,
    because: Vec<String>,
}

/// Collect the shown atoms of every model. Returns Err on any clingo failure so the caller
/// can map it to schema_invalid.
fn run_clingo(program: &str) -> Result<ShownAtoms, String> {
    // `--warn=none`: mirror solve.py's silent logger so a malformed ruleset (which we map
    // to schema_invalid) does not spew clingo warnings to stderr.
    let mut ctl = control(vec!["--warn=none".to_string()]).map_err(|e| e.to_string())?;
    ctl.add("base", &[], program).map_err(|e| e.to_string())?;
    let part = Part::new("base", vec![]).map_err(|e| e.to_string())?;
    ctl.ground(&[part]).map_err(|e| e.to_string())?;

    let mut atoms = ShownAtoms::default();

    let mut handle = ctl
        .solve(SolveMode::YIELD, &[])
        .map_err(|e| e.to_string())?;
    loop {
        handle.resume().map_err(|e| e.to_string())?;
        match handle.model() {
            Ok(Some(model)) => {
                let symbols = model.symbols(ShowType::SHOWN).map_err(|e| e.to_string())?;
                for sym in symbols {
                    let s = sym.to_string();
                    if let Some(inner) = s
                        .strip_prefix("decision(")
                        .and_then(|x| x.strip_suffix(')'))
                    {
                        atoms.decisions.push(inner.to_string());
                    } else if let Some(inner) =
                        s.strip_prefix("cite(").and_then(|x| x.strip_suffix(')'))
                    {
                        atoms.cites.push(inner.trim_matches('"').to_string());
                    } else if let Some(inner) =
                        s.strip_prefix("because(").and_then(|x| x.strip_suffix(')'))
                    {
                        atoms.because.push(inner.to_string());
                    } else if let Some(inner) = s
                        .strip_prefix("because_not(")
                        .and_then(|x| x.strip_suffix(')'))
                    {
                        atoms.because.push(inner.to_string());
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                handle.close().ok();
                return Err(e.to_string());
            }
        }
    }
    handle.close().ok();
    Ok(atoms)
}

fn sorted_dedup(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v.dedup();
    v
}

/// Solve a fact set against a ruleset. Mirrors `solve()` in solve.py:
/// exactly one decision → solved; several → conflict; none → abstain; any clingo
/// error → schema_invalid. Never panics.
pub fn solve(ruleset: &str, facts: &[String]) -> SolverResult {
    let fact_block: String = facts
        .iter()
        .map(|f| normalize(f))
        .filter(|f| !f.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let program = format!("{ruleset}\n{fact_block}");

    let atoms = match run_clingo(&program) {
        Ok(a) => a,
        Err(_) => {
            return SolverResult {
                decision: None,
                status: STATUS_SCHEMA_INVALID.to_string(),
                citations: vec![],
                justification: vec![],
            };
        }
    };

    let decisions = sorted_dedup(atoms.decisions);
    match decisions.len() {
        1 => SolverResult {
            decision: Some(decisions[0].clone()),
            status: STATUS_SOLVED.to_string(),
            citations: sorted_dedup(atoms.cites),
            justification: sorted_dedup(atoms.because),
        },
        n if n > 1 => SolverResult {
            decision: None,
            status: STATUS_CONFLICT.to_string(),
            citations: vec![],
            justification: vec![],
        },
        _ => SolverResult {
            decision: None,
            status: STATUS_ABSTAIN.to_string(),
            citations: vec![],
            justification: vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::topics::load_topics;
    use std::fs;
    use std::path::Path;

    fn rules_root() -> String {
        format!("{}/resources/rules", env!("CARGO_MANIFEST_DIR"))
    }

    fn read_rules(dir: &str) -> String {
        fs::read_to_string(Path::new(&rules_root()).join(dir).join("rules.lp"))
            .expect("read rules.lp")
    }

    #[derive(serde::Deserialize)]
    struct GoldExpect {
        status: String,
        decision: Option<String>,
        #[serde(default)]
        citations: Option<Vec<String>>,
    }

    #[derive(serde::Deserialize)]
    struct GoldCase {
        id: String,
        facts: Vec<String>,
        expect: GoldExpect,
    }

    fn read_gold(dir: &str) -> Vec<GoldCase> {
        let path = Path::new(&rules_root()).join(dir).join("gold/cases.jsonl");
        let text = fs::read_to_string(&path).expect("read gold cases");
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str::<GoldCase>(l).expect("parse gold case"))
            .collect()
    }

    // solve-vs-gold: clingo-rs must reproduce the Python baseline for every gold case
    // in BOTH topics. Proves the engine port is faithful.
    fn run_gold_for(dir: &str) {
        let rules = read_rules(dir);
        for case in read_gold(dir) {
            let r = solve(&rules, &case.facts);
            assert_eq!(
                r.status, case.expect.status,
                "[{dir}/{}] status mismatch: got {:?} want {:?}",
                case.id, r.status, case.expect.status
            );
            if case.expect.status == STATUS_SOLVED {
                assert_eq!(
                    r.decision, case.expect.decision,
                    "[{dir}/{}] decision mismatch",
                    case.id
                );
                if let Some(want) = &case.expect.citations {
                    let got: std::collections::BTreeSet<&String> = r.citations.iter().collect();
                    let exp: std::collections::BTreeSet<&String> = want.iter().collect();
                    assert_eq!(got, exp, "[{dir}/{}] citations mismatch", case.id);
                }
            }
        }
    }

    #[test]
    fn solve_vs_gold_reverse_charge() {
        run_gold_for("dph-reverse-charge");
    }

    #[test]
    fn solve_vs_gold_registration() {
        run_gold_for("dph-registration");
    }

    // Every topic's gold cases solve faithfully when loaded through the registry too.
    #[test]
    fn solve_vs_gold_all_loaded_topics() {
        let topics = load_topics().expect("load topics");
        assert!(!topics.is_empty());
        for t in &topics {
            let dir = &t.id;
            run_gold_for(dir);
        }
    }

    // Fail-closed: a syntactically broken ruleset must yield schema_invalid, not panic.
    // NOTE: clingo's C core prints its own "syntax error" line to stderr here; that is the
    // grounder rejecting the bad program (exactly what we want), not a Rust failure. The
    // clingo-rs 0.8 `control()` helper installs no logger, so unlike the Python sidecar
    // (which passes a no-op logger) we cannot silence that one diagnostic without switching
    // to the heavier `control_with_context` API. The test still asserts the correct result.
    #[test]
    fn broken_rules_are_schema_invalid() {
        let r = solve("this is not :- valid asp (((", &["foo(bar)".to_string()]);
        assert_eq!(r.status, STATUS_SCHEMA_INVALID);
        assert!(r.decision.is_none());
        assert!(r.citations.is_empty());
    }

    // Two decisions from contradictory facts → conflict.
    #[test]
    fn two_decisions_is_conflict() {
        let rules = "decision(a) :- p. decision(b) :- q. #show decision/1.";
        let r = solve(rules, &["p".to_string(), "q".to_string()]);
        assert_eq!(r.status, STATUS_CONFLICT);
        assert!(r.decision.is_none());
    }
}
