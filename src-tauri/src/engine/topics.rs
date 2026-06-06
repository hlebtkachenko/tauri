// topics.rs — the topic-module registry. Each accounting area is a directory under
// resources/rules/ with a manifest.json + rules.lp. Adding an area is a drop-in; the
// engine reads the slot vocabulary from data, never from hard-coded constants.

use std::fs;
use std::path::{Path, PathBuf};

use super::types::{EngineError, Topic};

/// Root of the bundled rules modules. For tests/dev this resolves relative to the crate;
/// the Tauri-resource path is a later phase.
fn rules_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("rules")
}

fn load_from(dir: &Path) -> Result<Vec<Topic>, EngineError> {
    let mut topics: Vec<Topic> = Vec::new();
    let entries = fs::read_dir(dir)
        .map_err(|e| EngineError::Topics(format!("read_dir {}: {e}", dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(|e| EngineError::Topics(e.to_string()))?;
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let module_dir = entry.path();
        let manifest_path = module_dir.join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&manifest_path)
            .map_err(|e| EngineError::Topics(format!("read {}: {e}", manifest_path.display())))?;
        let mut topic: Topic = serde_json::from_str(&raw)
            .map_err(|e| EngineError::Topics(format!("parse {}: {e}", manifest_path.display())))?;
        topic.rules_path = module_dir.join("rules.lp").to_string_lossy().into_owned();
        topics.push(topic);
    }
    // Deterministic order so callers (router, tests) see a stable list.
    topics.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(topics)
}

/// Load every topic module under resources/rules/.
pub fn load_topics() -> Result<Vec<Topic>, EngineError> {
    load_from(&rules_dir())
}

/// Find one topic by id.
pub fn get_topic(id: &str) -> Result<Option<Topic>, EngineError> {
    Ok(load_topics()?.into_iter().find(|t| t.id == id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_both_topics_with_ids_and_vocab() {
        let topics = load_topics().expect("load topics");
        let ids: Vec<&str> = topics.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"dph-reverse-charge"), "ids: {ids:?}");
        assert!(ids.contains(&"dph-registration"), "ids: {ids:?}");

        let rc = get_topic("dph-reverse-charge")
            .expect("get_topic")
            .expect("present");
        assert_eq!(rc.ruleset, "dph_reverse_charge");
        // Slot vocabulary comes from data.
        assert!(rc.slots.contains_key("supplier_vat"));
        assert!(rc.slots.contains_key("supply_category"));
        let cat = &rc.slots["supply_category"];
        assert_eq!(cat.enum_, vec!["construction_assembly", "other"]);
        assert_eq!(cat.fact_template, "supply_category({})");
        // rules_path resolved at load time.
        assert!(rc.rules_path.ends_with("dph-reverse-charge/rules.lp"));

        let reg = get_topic("dph-registration")
            .expect("get_topic")
            .expect("present");
        assert_eq!(reg.slots.len(), 1);
        assert!(reg.slots.contains_key("turnover_over_threshold"));
    }

    #[test]
    fn unknown_topic_is_none() {
        assert!(get_topic("nope").expect("get_topic").is_none());
    }
}
