// classify.rs — the request path, ported from the TS reference `src/classify.ts` +
// `src/local.ts`. One async function the Tauri `ask` command calls:
//
//   ask(store, question, as_of_date, on_progress) → Outcome
//
//   route(question) → None ⇒ gate_failure("off_topic")
//   else read trusted few-shot via store.trusted_by_tags(topic.tags, RETRIEVE_K)
//   extract_for_topic(question, topic, examples) → None/Err ⇒ abstain
//   solve(ruleset, facts) → gate → Outcome
//   append an Episode to the store (best effort).
//
// FAIL-CLOSED: any error → an abstain Outcome (never panic, never throw to the UI). A
// progress string is emitted per stage via the Channel.
//
// CONCURRENCY: the store is `!Send` (rusqlite Connection) and must never be held across an
// `.await`. The caller (the Tauri command) owns the store; here we take a synchronous
// closure for the trusted-retrieval read and another for the episode append, each of which
// locks → does its blocking work → drops the guard, with no await inside. The async LLM
// work (route/extract) happens between those two closures, holding no lock.

use std::fs;
use std::path::Path;

use super::budget::Budget;
use super::extract::extract_for_topic;
use super::gate::{gate, gate_failure};
use super::route::route;
use super::solve::solve;
use super::types::{EngineError, EpisodeInput, Outcome, StrategyItem, Topic};

/// k≈1 tag-matched retrieval (ReasoningBank: more retrieved hurts). Overridable via env.
fn retrieve_k() -> usize {
    std::env::var("RETRIEVE_K")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
}

/// Emit a progress string for a pipeline stage. The closure is the Channel `send`, made
/// infallible at this seam (a dropped progress message must never fail an answer). `Send +
/// Sync` so the `ask` future stays `Send` (Tauri's async command runtime requires it).
type Progress<'a> = dyn Fn(&str) + Send + Sync + 'a;

/// The ask pipeline. `retrieve` is the synchronous store read (locks, reads trusted few-shot,
/// drops the guard — NO await inside); `record` is the synchronous episode append (same
/// rule). Between them only async LLM/solver work runs, holding no store lock.
///
/// Fail-closed: every error path returns an `Outcome::Abstain` (via `gate_failure`); nothing
/// here can panic on the hot path.
pub async fn ask<R, W>(
    question: &str,
    as_of_date: &str,
    on_progress: &Progress<'_>,
    retrieve: R,
    record: W,
) -> Outcome
where
    R: Fn(&[String], usize) -> Result<Vec<StrategyItem>, EngineError> + Send,
    W: Fn(EpisodeInput) + Send,
{
    let mut budget = Budget::from_env();

    on_progress("route");
    let topic: Topic = match route(question, &mut budget).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            let out = gate_failure("off_topic");
            record(episode_for(None, question, as_of_date, None, &out));
            return out;
        }
        Err(e) => {
            let out = gate_failure(&e.to_string());
            record(episode_for(None, question, as_of_date, None, &out));
            return out;
        }
    };

    on_progress("retrieve");
    let examples: Vec<StrategyItem> = retrieve(&topic.tags, retrieve_k()).unwrap_or_default();
    let example_pairs: Vec<(String, String)> = examples
        .iter()
        .map(|e| (e.title.clone(), e.content.clone()))
        .collect();

    on_progress("extract");
    let facts: Vec<String> =
        match extract_for_topic(question, &topic, &example_pairs, &mut budget).await {
            Ok(f) => f,
            Err(_) => {
                let out = gate_failure("extraction_failed");
                record(episode_for(
                    Some(&topic.id),
                    question,
                    as_of_date,
                    None,
                    &out,
                ));
                return out;
            }
        };

    on_progress("solve");
    let ruleset = match fs::read_to_string(Path::new(&topic.rules_path)) {
        Ok(r) => r,
        Err(e) => {
            let out = gate_failure(&format!("ruleset_read:{e}"));
            record(episode_for(
                Some(&topic.id),
                question,
                as_of_date,
                Some(&facts),
                &out,
            ));
            return out;
        }
    };
    let result = solve(&ruleset, &facts);

    on_progress("gate");
    let out = gate(&result);
    let mut episode = episode_for(Some(&topic.id), question, as_of_date, Some(&facts), &out);
    // The episode status records the solver verdict (solved/abstain/conflict/schema_invalid),
    // not just answer/abstain, mirroring the TS trace.
    episode.status = result.status.clone();
    record(episode);
    out
}

/// Build the `EpisodeInput` captured for every ask (the raw material a correction attaches
/// to). `decision`/`citations` come from the outcome; `status` defaults to the outcome kind
/// and is overridden by the caller when the solver status is known.
fn episode_for(
    topic: Option<&str>,
    question: &str,
    as_of_date: &str,
    facts: Option<&[String]>,
    out: &Outcome,
) -> EpisodeInput {
    let (decision, status, citations) = match out {
        Outcome::Answer {
            decision,
            citations,
            ..
        } => (
            Some(decision.clone()),
            "solved".to_string(),
            citations.clone(),
        ),
        Outcome::Abstain { reason } => (None, reason.clone(), Vec::new()),
    };
    EpisodeInput {
        topic: topic.map(|t| t.to_string()),
        question: question.to_string(),
        as_of_date: as_of_date.to_string(),
        facts: facts.map(|f| f.to_vec()),
        decision,
        status,
        citations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Live end-to-end smoke test: drives the real `claude` CLI on the subscription through the
    // full Rust pipeline (route → extract → solve → gate). Ignored by default (costs money +
    // needs auth); run with: cargo test --manifest-path src-tauri/Cargo.toml live_ask -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "live: spawns the claude CLI on the OAuth subscription"]
    async fn live_ask_construction() {
        let on_progress = |stage: &str| println!("[stage] {stage}");
        let retrieve = |_: &[String], _: usize| Ok(Vec::new());
        let record = |_: EpisodeInput| {};
        let out = ask(
            "A Czech VAT-payer does construction-assembly work for another Czech VAT-payer in Prague",
            "2026-06-06",
            &on_progress,
            retrieve,
            record,
        )
        .await;
        println!("OUTCOME: {out:?}");
        match out {
            Outcome::Answer {
                decision,
                citations,
                ..
            } => {
                assert_eq!(decision, "reverse_charge_applies");
                assert!(citations.contains(&"§92a".to_string()));
                assert!(citations.contains(&"§92e".to_string()));
            }
            Outcome::Abstain { reason } => panic!("expected answer, got abstain: {reason}"),
        }
    }
}
