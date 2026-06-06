// route.rs — fail-closed topic router: question → Option<Topic> (None ⇒ abstain).
//
// Single registered topic ⇒ passthrough, NO extra LLM call: the extractor's
// "unknown"-fence already abstains on off-topic cases. With ≥2 topics the LLM classifier
// activates and picks one topic id or "unknown" → None.
//
// NOT unit-tested here (the ≥2-topic branch calls the live CLI).

use super::budget::Budget;
use super::claude::claude_json;
use super::topics::load_topics;
use super::types::{EngineError, Topic};

fn parse_topic_id(text: &str) -> Option<String> {
    let json = crate::engine::extract::extract_json_object_pub(text)?;
    let raw: serde_json::Value = serde_json::from_str(&json).ok()?;
    raw.get("topic")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Route a question to exactly one topic, or None (⇒ abstain).
pub async fn route(question: &str, budget: &mut Budget) -> Result<Option<Topic>, EngineError> {
    let topics = load_topics()?;
    if topics.is_empty() {
        return Ok(None);
    }
    if topics.len() == 1 {
        // exactly one topic: passthrough; `.next()` yields it (None only if empty → abstain).
        return Ok(topics.into_iter().next());
    }

    budget.charge_call()?;
    let list = topics
        .iter()
        .map(|t| format!("- {}: {}", t.id, t.description))
        .collect::<Vec<_>>()
        .join("\n");
    let model =
        std::env::var("MODEL_UTILITY").unwrap_or_else(|_| "claude-haiku-4-5-20251001".to_string());
    let prompt = format!(
        "Topics:\n{list}\n\nWhich single topic does this accounting question fall under? \
         Answer \"unknown\" if none clearly fits.\n\nQuestion:\n{question}"
    );
    let system = "You route a Czech accounting question to exactly one topic id, or \
        \"unknown\" if none clearly applies. Reason briefly, then output a JSON object in a \
        ```json fence with key \"topic\" set to one of the topic ids or \"unknown\".";

    let res = claude_json(&prompt, system, &model).await?;
    budget.record_cost(res.cost_usd);
    if res.is_error {
        return Ok(None);
    }
    let id = match parse_topic_id(&res.result) {
        Some(id) if id != "unknown" => id,
        _ => return Ok(None),
    };
    Ok(topics.into_iter().find(|t| t.id == id))
}
