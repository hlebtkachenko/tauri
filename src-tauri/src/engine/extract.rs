// extract.rs — neural side. Maps a case onto a topic's FIXED slot vocabulary using the
// local `claude` CLI (see claude.rs). The vocabulary comes from the topic manifest (data),
// so the model can never emit an out-of-vocabulary predicate and a new accounting area
// needs no change here.
//
// Reason-then-emit: the system prompt asks the model to reason in free text first, then
// emit the slots as a JSON object in a ```json fence. We parse that object and then FENCE
// every value to the enum deterministically — never trusting the model's output.
//
// Selective prediction: with EXTRACT_SAMPLES > 1 we sample N independent extractions and
// keep a slot only if all N agree; any disagreement → "unknown" (→ likely abstain).
//
// Fail-closed: spawn/timeout/parse failure, is_error, or no JSON on ANY sample → Err
// (caller abstains).

use std::collections::BTreeMap;

use super::budget::Budget;
use super::claude::claude_json;
use super::types::{EngineError, Topic};

const UNKNOWN: &str = "unknown";

/// Reason-then-emit instruction generated from the topic's slot vocabulary (data-driven).
fn slot_instruction(topic: &Topic) -> String {
    let lines = topic
        .slots
        .iter()
        .map(|(slot, spec)| {
            let mut allowed: Vec<String> = spec.enum_.iter().map(|v| format!("\"{v}\"")).collect();
            allowed.push(format!("\"{UNKNOWN}\""));
            format!("  \"{slot}\": {}", allowed.join(" | "))
        })
        .collect::<Vec<_>>()
        .join("\n");
    [
        "First reason briefly in plain text about each field the case actually states.",
        "Then output the final answer as a JSON object inside a ```json code block, with",
        "EXACTLY these keys and one of the allowed values each:",
        "{",
        &lines,
        "}",
        "Use \"unknown\" for any field the case does not clearly and explicitly state.",
    ]
    .join("\n")
}

/// Recover the JSON object: prefer the LAST ```json fenced block, else the last {...}.
fn extract_json_object(text: &str) -> Option<String> {
    if let Some(block) = last_fenced_block(text) {
        return Some(block);
    }
    last_brace_object(text)
}

/// Same fenced-JSON recovery, exposed for the router (which reads a `{"topic": ...}` blob).
pub fn extract_json_object_pub(text: &str) -> Option<String> {
    extract_json_object(text)
}

// Find the last ```...``` (optionally ```json) fenced block's inner content, trimmed.
fn last_fenced_block(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut search_from = 0usize;
    let mut last: Option<String> = None;
    while let Some(rel) = text[search_from..].find("```") {
        let open = search_from + rel;
        // Skip the opening fence and an optional language tag up to the newline.
        let after_fence = open + 3;
        let content_start = match text[after_fence..].find('\n') {
            Some(nl) => after_fence + nl + 1,
            None => break,
        };
        // The closing fence.
        let close = match text[content_start..].find("```") {
            Some(c) => content_start + c,
            None => break,
        };
        last = Some(text[content_start..close].trim().to_string());
        search_from = close + 3;
        if search_from >= bytes.len() {
            break;
        }
    }
    last
}

// The last {...} span (greedy from the last '{' to the last '}').
fn last_brace_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end >= start {
        Some(text[start..=end].to_string())
    } else {
        None
    }
}

/// Parse the model output into fenced slot values. Returns None when no JSON object is
/// present or it does not parse. Every slot is forced to a known enum value or "unknown".
pub fn parse_slots(topic: &Topic, text: &str) -> Option<BTreeMap<String, String>> {
    let json = extract_json_object(text)?;
    let raw: serde_json::Value = serde_json::from_str(&json).ok()?;
    let obj = raw.as_object()?;
    let mut slots: BTreeMap<String, String> = BTreeMap::new();
    for (slot, spec) in &topic.slots {
        let v = obj.get(slot).and_then(|x| x.as_str());
        let fenced = match v {
            Some(s) if spec.enum_.iter().any(|e| e == s) => s.to_string(),
            _ => UNKNOWN.to_string(),
        };
        slots.insert(slot.clone(), fenced);
    }
    Some(slots)
}

/// Turn fenced slots into ASP facts. Skips "unknown" and any out-of-enum value.
pub fn slots_to_facts(topic: &Topic, slots: &BTreeMap<String, String>) -> Vec<String> {
    let mut facts: Vec<String> = Vec::new();
    for (slot, spec) in &topic.slots {
        if let Some(v) = slots.get(slot) {
            if v != UNKNOWN && spec.enum_.iter().any(|e| e == v) {
                facts.push(spec.fact_template.replace("{}", v));
            }
        }
    }
    facts
}

/// One extraction sample → fenced slots, or Err on any failure (fail-closed).
async fn sample_slots(
    prompt: &str,
    system: &str,
    topic: &Topic,
    model: &str,
    budget: &mut Budget,
) -> Result<BTreeMap<String, String>, EngineError> {
    budget.charge_call()?;
    let res = claude_json(prompt, system, model).await?;
    budget.record_cost(res.cost_usd);
    if res.is_error {
        return Err(EngineError::Claude("is_error".to_string()));
    }
    parse_slots(topic, &res.result)
        .ok_or_else(|| EngineError::Claude("no parseable JSON slots".to_string()))
}

/// Extract ASP facts for a topic from a free-text case. Samples EXTRACT_SAMPLES times and
/// keeps a slot only if every sample agrees; any failed sample → Err (caller abstains).
/// NOT unit-tested here (it calls the live CLI) — the pure helpers above are.
pub async fn extract_for_topic(
    question: &str,
    topic: &Topic,
    examples: &[(String, String)], // (title, content) prior lessons
    budget: &mut Budget,
) -> Result<Vec<String>, EngineError> {
    let system = format!("{}\n\n{}", topic.system_prompt, slot_instruction(topic));
    let fewshot = examples
        .iter()
        .map(|(title, content)| format!("- {title}: {content}"))
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = if fewshot.is_empty() {
        format!("Case:\n{question}")
    } else {
        format!("Prior lessons:\n{fewshot}\n\nCase:\n{question}")
    };

    let model =
        std::env::var("MODEL_UTILITY").unwrap_or_else(|_| "claude-haiku-4-5-20251001".to_string());
    let samples = std::env::var("EXTRACT_SAMPLES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);

    let mut runs: Vec<BTreeMap<String, String>> = Vec::with_capacity(samples);
    for _ in 0..samples {
        runs.push(sample_slots(&prompt, &system, topic, &model, budget).await?);
    }

    // Keep a slot only if every sample agrees; otherwise "unknown".
    let mut merged: BTreeMap<String, String> = BTreeMap::new();
    for slot in topic.slots.keys() {
        let first = runs[0]
            .get(slot)
            .cloned()
            .unwrap_or_else(|| UNKNOWN.to_string());
        let all_agree = runs
            .iter()
            .all(|r| r.get(slot).map(|v| v == &first).unwrap_or(false));
        merged.insert(
            slot.clone(),
            if all_agree {
                first
            } else {
                UNKNOWN.to_string()
            },
        );
    }
    Ok(slots_to_facts(topic, &merged))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::topics::get_topic;

    fn rc_topic() -> Topic {
        get_topic("dph-reverse-charge")
            .expect("get_topic")
            .expect("present")
    }

    // Enum fence: an out-of-enum value → "unknown"; valid values pass through.
    #[test]
    fn enum_fence_rejects_out_of_enum() {
        let topic = rc_topic();
        let text = r#"reasoning...
```json
{
  "supplier_vat": "payer",
  "customer_vat": "BOGUS",
  "place_of_supply": "czech_republic",
  "supply_category": "construction_assembly"
}
```"#;
        let slots = parse_slots(&topic, text).expect("parsed");
        assert_eq!(slots["supplier_vat"], "payer");
        assert_eq!(slots["customer_vat"], "unknown"); // fenced out
        assert_eq!(slots["place_of_supply"], "czech_republic");
        assert_eq!(slots["supply_category"], "construction_assembly");
    }

    // A missing key → "unknown".
    #[test]
    fn missing_key_becomes_unknown() {
        let topic = rc_topic();
        let text = r#"```json
{ "supplier_vat": "payer" }
```"#;
        let slots = parse_slots(&topic, text).expect("parsed");
        assert_eq!(slots["supplier_vat"], "payer");
        assert_eq!(slots["customer_vat"], "unknown");
    }

    // slotsToFacts builds the right ASP facts and skips "unknown".
    #[test]
    fn slots_to_facts_builds_asp_and_skips_unknown() {
        let topic = rc_topic();
        let mut slots: BTreeMap<String, String> = BTreeMap::new();
        slots.insert("supplier_vat".to_string(), "payer".to_string());
        slots.insert("customer_vat".to_string(), "payer".to_string());
        slots.insert("place_of_supply".to_string(), "czech_republic".to_string());
        slots.insert("supply_category".to_string(), "unknown".to_string());
        let facts = slots_to_facts(&topic, &slots);
        assert!(facts.contains(&"vat_status(supplier, payer)".to_string()));
        assert!(facts.contains(&"vat_status(customer, payer)".to_string()));
        assert!(facts.contains(&"place_of_supply(czech_republic)".to_string()));
        // supply_category is unknown → no fact emitted.
        assert!(!facts.iter().any(|f| f.starts_with("supply_category")));
        assert_eq!(facts.len(), 3);
    }

    // The LAST fenced block wins (model may reason in an earlier fence).
    #[test]
    fn last_fenced_block_wins() {
        let topic = rc_topic();
        let text = r#"```json
{ "supplier_vat": "nonpayer" }
```
then I reconsidered:
```json
{ "supplier_vat": "payer", "customer_vat": "payer", "place_of_supply": "czech_republic", "supply_category": "other" }
```"#;
        let slots = parse_slots(&topic, text).expect("parsed");
        assert_eq!(slots["supplier_vat"], "payer");
        assert_eq!(slots["supply_category"], "other");
    }

    // Fallback to the last {...} when there is no fence.
    #[test]
    fn falls_back_to_brace_object() {
        let topic = rc_topic();
        let text = r#"My answer: { "turnover_over_threshold": "yes" } done."#;
        let reg = get_topic("dph-registration")
            .expect("get_topic")
            .expect("present");
        let _ = topic; // rc topic unused here
        let slots = parse_slots(&reg, text).expect("parsed");
        assert_eq!(slots["turnover_over_threshold"], "yes");
    }

    // No JSON at all → None (caller abstains).
    #[test]
    fn no_json_is_none() {
        let topic = rc_topic();
        assert!(parse_slots(&topic, "no json here, sorry").is_none());
    }

    // slot_instruction lists every slot with its enum plus "unknown".
    #[test]
    fn slot_instruction_lists_vocab_with_unknown() {
        let topic = rc_topic();
        let instr = slot_instruction(&topic);
        assert!(instr.contains("\"supplier_vat\""));
        assert!(instr.contains("\"construction_assembly\""));
        assert!(instr.contains("\"unknown\""));
        assert!(instr.contains("```json"));
    }
}
