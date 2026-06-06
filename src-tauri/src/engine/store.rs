// store.rs — persistence for the learning loop, ported from the TS `Store` interface +
// `fileStore` onto embedded SQLite (rusqlite, bundled — no system sqlite, codesign-clean).
//
// The `Store` trait is the seam the Tauri commands (P3) call; `SqliteStore` is the one
// implementation, opening either a file path or an in-memory db. The schema ports
// db/0001_init.sql to SQLite: INTEGER/TEXT columns, ISO-8601 TEXT timestamps, JSON-encoded
// TEXT for arrays/objects, TEXT CHECK enums, pgvector dropped (v1 retrieval is tag-based,
// ADR-0006). A `budget` table persists the cost guard across runs (replaces P1's
// per-process Budget).
//
// rusqlite is SYNCHRONOUS: every method is a plain blocking `fn`. The Connection is !Send,
// so a SqliteStore must never be held across an `.await`. Fail-closed: every rusqlite/JSON
// error propagates as `Err(EngineError::Store(..))`; no unwrap/expect on any path.

use rusqlite::{params, Connection, OptionalExtension, Row};
use uuid::Uuid;

use super::types::{
    Correction, CorrectionInput, EngineError, Episode, EpisodeInput, GateResult, Provenance,
    StoredStrategyItem, StrategyItem, StrategyItemInput, StrategyItemPatch, TrustState,
};

/// The persistence seam. Methods are blocking (rusqlite is sync). P3's Tauri commands call
/// these off the async hot path (e.g. via `spawn_blocking`) so the !Send Connection never
/// crosses an await.
pub trait Store {
    fn append_episode(&self, e: EpisodeInput) -> Result<Episode, EngineError>;
    fn get_episode(&self, id: &str) -> Result<Option<Episode>, EngineError>;
    fn recent_episodes(&self, n: usize) -> Result<Vec<Episode>, EngineError>;
    fn append_correction(&self, c: CorrectionInput) -> Result<Correction, EngineError>;
    fn add_strategy_item(&self, s: StrategyItemInput) -> Result<StoredStrategyItem, EngineError>;
    fn update_strategy_item(
        &self,
        id: &str,
        patch: StrategyItemPatch,
    ) -> Result<Option<StoredStrategyItem>, EngineError>;
    fn list_strategy_items(
        &self,
        trust_state: Option<TrustState>,
    ) -> Result<Vec<StoredStrategyItem>, EngineError>;
    /// k≈1 tag-matched retrieval of TRUSTED lessons for few-shot injection (ReasoningBank:
    /// more retrieved hurts). The `_exclude` arg mirrors the reference seam (unused in v1).
    fn trusted_by_tags(
        &self,
        tags: &[String],
        exclude: &[String],
        k: usize,
    ) -> Result<Vec<StrategyItem>, EngineError>;

    // --- budget (persisted cost guard; replaces P1's per-process Budget) ---
    fn budget_get(&self) -> Result<(u64, f64), EngineError>;
    /// Pre-call gate: increments the call count and persists it. Caller enforces caps.
    fn budget_charge_call(&self) -> Result<(), EngineError>;
    /// Accumulate the actual cost reported by a completed call.
    fn budget_record_cost(&self, usd: f64) -> Result<(), EngineError>;
}

const SCHEMA: &str = "\
create table if not exists episode (
  id            text primary key,
  created_at    text not null,
  topic         text,
  question      text not null,
  as_of_date    text not null,
  facts         text,                                  -- JSON array or null
  decision      text,
  status        text not null,
  citations     text not null default '[]'             -- JSON array
);

create table if not exists correction (
  id                 text primary key,
  created_at         text not null,
  episode_id         text not null references episode(id),
  correction_type    text not null check (correction_type in ('fact_mapping','rule_defect','vocabulary_gap')),
  corrected_decision text,
  governing_section  text,
  expert             text not null,
  note               text
);

create table if not exists strategy_item (
  id           text primary key,
  created_at   text not null,
  title        text not null,
  description  text not null,
  content      text not null,
  tags         text not null default '[]',             -- JSON array
  trust_state  text not null default 'provisional' check (trust_state in ('provisional','trusted')),
  provenance   text not null,                          -- JSON {episode_id, correction_id, source}
  helpful      integer not null default 0,
  harmful      integer not null default 0,
  gate         text,                                   -- JSON GateResult or null
  approved_by  text,
  approved_at  text
);

create table if not exists budget (
  id        integer primary key check (id = 1),
  calls     integer not null default 0,
  spent_usd real not null default 0.0
);

create index if not exists strategy_item_trust_idx on strategy_item (trust_state);
create index if not exists episode_status_idx on episode (status);
";

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    /// Open (or create) a store at a file path, or `":memory:"` for a transient db.
    pub fn open(path: &str) -> Result<Self, EngineError> {
        let conn = Connection::open(path).map_err(store_err)?;
        Self::init(conn)
    }

    /// In-memory store (tests).
    pub fn in_memory() -> Result<Self, EngineError> {
        let conn = Connection::open_in_memory().map_err(store_err)?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self, EngineError> {
        conn.execute_batch(SCHEMA).map_err(store_err)?;
        conn.execute(
            "insert or ignore into budget (id, calls, spent_usd) values (1, 0, 0.0)",
            [],
        )
        .map_err(store_err)?;
        Ok(SqliteStore { conn })
    }
}

fn store_err<E: std::fmt::Display>(e: E) -> EngineError {
    EngineError::Store(e.to_string())
}

// JSON helpers — arrays/objects are stored as TEXT.
fn to_json<T: serde::Serialize>(v: &T) -> Result<String, EngineError> {
    serde_json::to_string(v).map_err(store_err)
}

fn from_json<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, EngineError> {
    serde_json::from_str(s).map_err(store_err)
}

fn opt_json_array(s: Option<String>) -> Result<Option<Vec<String>>, EngineError> {
    match s {
        Some(text) => Ok(Some(from_json::<Vec<String>>(&text)?)),
        None => Ok(None),
    }
}

fn row_to_episode(row: &Row) -> Result<Episode, rusqlite::Error> {
    Ok(Episode {
        id: row.get("id")?,
        created_at: row.get("created_at")?,
        topic: row.get("topic")?,
        question: row.get("question")?,
        as_of_date: row.get("as_of_date")?,
        // facts / citations are JSON TEXT; deserialize after the row borrow.
        facts: None,
        decision: row.get("decision")?,
        status: row.get("status")?,
        citations: vec![],
    })
}

impl Store for SqliteStore {
    fn append_episode(&self, e: EpisodeInput) -> Result<Episode, EngineError> {
        let id = Uuid::new_v4().to_string();
        let facts_json = match &e.facts {
            Some(f) => Some(to_json(f)?),
            None => None,
        };
        let citations_json = to_json(&e.citations)?;
        self.conn
            .execute(
                "insert into episode
                   (id, created_at, topic, question, as_of_date, facts, decision, status, citations)
                 values (?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    id,
                    e.topic,
                    e.question,
                    e.as_of_date,
                    facts_json,
                    e.decision,
                    e.status,
                    citations_json,
                ],
            )
            .map_err(store_err)?;
        self.get_episode(&id)?
            .ok_or_else(|| EngineError::Store("episode vanished after insert".to_string()))
    }

    fn get_episode(&self, id: &str) -> Result<Option<Episode>, EngineError> {
        let mut stmt = self
            .conn
            .prepare(
                "select id, created_at, topic, question, as_of_date, facts, decision, status, citations
                   from episode where id = ?1",
            )
            .map_err(store_err)?;
        let row = stmt
            .query_row(params![id], |r| {
                let mut ep = row_to_episode(r)?;
                let facts: Option<String> = r.get("facts")?;
                let citations: String = r.get("citations")?;
                Ok((ep_take(&mut ep), facts, citations))
            })
            .optional()
            .map_err(store_err)?;
        match row {
            Some((mut ep, facts, citations)) => {
                ep.facts = opt_json_array(facts)?;
                ep.citations = from_json::<Vec<String>>(&citations)?;
                Ok(Some(ep))
            }
            None => Ok(None),
        }
    }

    fn recent_episodes(&self, n: usize) -> Result<Vec<Episode>, EngineError> {
        let mut stmt = self
            .conn
            .prepare(
                "select id, created_at, topic, question, as_of_date, facts, decision, status, citations
                   from episode order by rowid desc limit ?1",
            )
            .map_err(store_err)?;
        let rows = stmt
            .query_map(params![n as i64], |r| {
                let mut ep = row_to_episode(r)?;
                let facts: Option<String> = r.get("facts")?;
                let citations: String = r.get("citations")?;
                Ok((ep_take(&mut ep), facts, citations))
            })
            .map_err(store_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (mut ep, facts, citations) = row.map_err(store_err)?;
            ep.facts = opt_json_array(facts)?;
            ep.citations = from_json::<Vec<String>>(&citations)?;
            out.push(ep);
        }
        Ok(out)
    }

    fn append_correction(&self, c: CorrectionInput) -> Result<Correction, EngineError> {
        let id = Uuid::new_v4().to_string();
        self.conn
            .execute(
                "insert into correction
                   (id, created_at, episode_id, correction_type, corrected_decision,
                    governing_section, expert, note)
                 values (?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    id,
                    c.episode_id,
                    c.correction_type.as_str(),
                    c.corrected_decision,
                    c.governing_section,
                    c.expert,
                    c.note,
                ],
            )
            .map_err(store_err)?;
        self.get_correction(&id)?
            .ok_or_else(|| EngineError::Store("correction vanished after insert".to_string()))
    }

    fn add_strategy_item(&self, s: StrategyItemInput) -> Result<StoredStrategyItem, EngineError> {
        let id = Uuid::new_v4().to_string();
        let tags_json = to_json(&s.tags)?;
        let provenance_json = to_json(&s.provenance)?;
        let gate_json = match &s.gate {
            Some(g) => Some(to_json(g)?),
            None => None,
        };
        self.conn
            .execute(
                "insert into strategy_item
                   (id, created_at, title, description, content, tags, trust_state, provenance,
                    helpful, harmful, gate, approved_by, approved_at)
                 values (?1, datetime('now'), ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    id,
                    s.title,
                    s.description,
                    s.content,
                    tags_json,
                    s.trust_state.as_str(),
                    provenance_json,
                    s.helpful,
                    s.harmful,
                    gate_json,
                    s.approved_by,
                    s.approved_at,
                ],
            )
            .map_err(store_err)?;
        self.get_strategy_item(&id)?
            .ok_or_else(|| EngineError::Store("strategy item vanished after insert".to_string()))
    }

    fn update_strategy_item(
        &self,
        id: &str,
        patch: StrategyItemPatch,
    ) -> Result<Option<StoredStrategyItem>, EngineError> {
        let existing = match self.get_strategy_item(id)? {
            Some(item) => item,
            None => return Ok(None),
        };
        let trust_state = patch.trust_state.unwrap_or(existing.trust_state);
        let helpful = patch.helpful.unwrap_or(existing.helpful);
        let harmful = patch.harmful.unwrap_or(existing.harmful);
        let gate = patch.gate.or(existing.gate);
        let approved_by = patch.approved_by.or(existing.approved_by);
        let approved_at = patch.approved_at.or(existing.approved_at);
        let gate_json = match &gate {
            Some(g) => Some(to_json(g)?),
            None => None,
        };
        self.conn
            .execute(
                "update strategy_item set
                   trust_state = ?2, helpful = ?3, harmful = ?4,
                   gate = ?5, approved_by = ?6, approved_at = ?7
                 where id = ?1",
                params![
                    id,
                    trust_state.as_str(),
                    helpful,
                    harmful,
                    gate_json,
                    approved_by,
                    approved_at,
                ],
            )
            .map_err(store_err)?;
        self.get_strategy_item(id)
    }

    fn list_strategy_items(
        &self,
        trust_state: Option<TrustState>,
    ) -> Result<Vec<StoredStrategyItem>, EngineError> {
        let (sql, filter): (&str, Option<&'static str>) = match trust_state {
            Some(ts) => (
                "select id from strategy_item where trust_state = ?1 order by rowid asc",
                Some(ts.as_str()),
            ),
            None => ("select id from strategy_item order by rowid asc", None),
        };
        let mut stmt = self.conn.prepare(sql).map_err(store_err)?;
        let ids: Vec<String> = match filter {
            Some(ts) => stmt
                .query_map(params![ts], |r| r.get::<_, String>(0))
                .map_err(store_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(store_err)?,
            None => stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(store_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(store_err)?,
        };
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(item) = self.get_strategy_item(&id)? {
                out.push(item);
            }
        }
        Ok(out)
    }

    fn trusted_by_tags(
        &self,
        tags: &[String],
        _exclude: &[String],
        k: usize,
    ) -> Result<Vec<StrategyItem>, EngineError> {
        // Only TRUSTED items with at least one overlapping tag, ranked by (helpful-harmful)
        // descending, capped at k. Tag overlap is computed in Rust (tags are JSON TEXT).
        let want: std::collections::HashSet<&str> = tags.iter().map(|s| s.as_str()).collect();
        let items = self.list_strategy_items(Some(TrustState::Trusted))?;
        let mut matched: Vec<StoredStrategyItem> = items
            .into_iter()
            .filter(|it| it.tags.iter().any(|t| want.contains(t.as_str())))
            .collect();
        matched.sort_by(|a, b| {
            let sa = a.helpful - a.harmful;
            let sb = b.helpful - b.harmful;
            sb.cmp(&sa)
        });
        Ok(matched
            .into_iter()
            .take(k)
            .map(|it| it.to_strategy_item())
            .collect())
    }

    fn budget_get(&self) -> Result<(u64, f64), EngineError> {
        self.conn
            .query_row(
                "select calls, spent_usd from budget where id = 1",
                [],
                |r| {
                    let calls: i64 = r.get(0)?;
                    let spent: f64 = r.get(1)?;
                    Ok((calls.max(0) as u64, spent))
                },
            )
            .map_err(store_err)
    }

    fn budget_charge_call(&self) -> Result<(), EngineError> {
        self.conn
            .execute("update budget set calls = calls + 1 where id = 1", [])
            .map_err(store_err)?;
        Ok(())
    }

    fn budget_record_cost(&self, usd: f64) -> Result<(), EngineError> {
        if usd.is_finite() && usd > 0.0 {
            self.conn
                .execute(
                    "update budget set spent_usd = spent_usd + ?1 where id = 1",
                    params![usd],
                )
                .map_err(store_err)?;
        }
        Ok(())
    }
}

impl SqliteStore {
    fn get_correction(&self, id: &str) -> Result<Option<Correction>, EngineError> {
        self.conn
            .query_row(
                "select id, created_at, episode_id, correction_type, corrected_decision,
                        governing_section, expert, note
                   from correction where id = ?1",
                params![id],
                |r| {
                    let type_str: String = r.get("correction_type")?;
                    Ok((
                        Correction {
                            id: r.get("id")?,
                            created_at: r.get("created_at")?,
                            episode_id: r.get("episode_id")?,
                            // placeholder; replaced below (needs fallible parse).
                            correction_type: super::types::CorrectionType::FactMapping,
                            corrected_decision: r.get("corrected_decision")?,
                            governing_section: r.get("governing_section")?,
                            expert: r.get("expert")?,
                            note: r.get("note")?,
                        },
                        type_str,
                    ))
                },
            )
            .optional()
            .map_err(store_err)?
            .map(|(mut c, type_str)| {
                c.correction_type = super::types::CorrectionType::from_str(&type_str)?;
                Ok(c)
            })
            .transpose()
    }

    fn get_strategy_item(&self, id: &str) -> Result<Option<StoredStrategyItem>, EngineError> {
        let raw = self
            .conn
            .query_row(
                "select id, created_at, title, description, content, tags, trust_state,
                        provenance, helpful, harmful, gate, approved_by, approved_at
                   from strategy_item where id = ?1",
                params![id],
                |r| {
                    Ok(RawStrategy {
                        id: r.get("id")?,
                        created_at: r.get("created_at")?,
                        title: r.get("title")?,
                        description: r.get("description")?,
                        content: r.get("content")?,
                        tags: r.get("tags")?,
                        trust_state: r.get("trust_state")?,
                        provenance: r.get("provenance")?,
                        helpful: r.get("helpful")?,
                        harmful: r.get("harmful")?,
                        gate: r.get("gate")?,
                        approved_by: r.get("approved_by")?,
                        approved_at: r.get("approved_at")?,
                    })
                },
            )
            .optional()
            .map_err(store_err)?;
        match raw {
            Some(raw) => Ok(Some(raw.into_item()?)),
            None => Ok(None),
        }
    }
}

/// Raw strategy_item row before JSON/enum fields are decoded (keeps the row closure
/// infallible-on-our-types so decode errors surface as `EngineError::Store`).
struct RawStrategy {
    id: String,
    created_at: String,
    title: String,
    description: String,
    content: String,
    tags: String,
    trust_state: String,
    provenance: String,
    helpful: i64,
    harmful: i64,
    gate: Option<String>,
    approved_by: Option<String>,
    approved_at: Option<String>,
}

impl RawStrategy {
    fn into_item(self) -> Result<StoredStrategyItem, EngineError> {
        let tags: Vec<String> = from_json(&self.tags)?;
        let provenance: Provenance = from_json(&self.provenance)?;
        let trust_state = TrustState::from_str(&self.trust_state)?;
        let gate: Option<GateResult> = match self.gate {
            Some(g) => Some(from_json(&g)?),
            None => None,
        };
        Ok(StoredStrategyItem {
            id: self.id,
            created_at: self.created_at,
            title: self.title,
            description: self.description,
            content: self.content,
            tags,
            trust_state,
            provenance,
            helpful: self.helpful,
            harmful: self.harmful,
            gate,
            approved_by: self.approved_by,
            approved_at: self.approved_at,
        })
    }
}

/// Move an `Episode` out of a `&mut Episode` built inside a row closure (the JSON fields
/// are filled in afterwards). Cheap: the partial episode owns small Strings.
fn ep_take(ep: &mut Episode) -> Episode {
    Episode {
        id: std::mem::take(&mut ep.id),
        created_at: std::mem::take(&mut ep.created_at),
        topic: ep.topic.take(),
        question: std::mem::take(&mut ep.question),
        as_of_date: std::mem::take(&mut ep.as_of_date),
        facts: ep.facts.take(),
        decision: ep.decision.take(),
        status: std::mem::take(&mut ep.status),
        citations: std::mem::take(&mut ep.citations),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SqliteStore {
        SqliteStore::in_memory().expect("in-memory store")
    }

    fn ep_input() -> EpisodeInput {
        EpisodeInput {
            topic: Some("dph-reverse-charge".to_string()),
            question: "q".to_string(),
            as_of_date: "2026-01-01".to_string(),
            facts: Some(vec!["f".to_string()]),
            decision: Some("d".to_string()),
            status: "solved".to_string(),
            citations: vec!["§1".to_string()],
        }
    }

    // store: episode + correction round-trip.
    #[test]
    fn episodes_and_corrections_round_trip() {
        let s = store();
        let ep = s.append_episode(ep_input()).expect("append episode");
        assert!(!ep.id.is_empty());
        let got = s.get_episode(&ep.id).expect("get").expect("present");
        assert_eq!(got.question, "q");
        assert_eq!(got.facts, Some(vec!["f".to_string()]));
        assert_eq!(got.citations, vec!["§1".to_string()]);

        let c = s
            .append_correction(CorrectionInput {
                episode_id: ep.id.clone(),
                correction_type: super::super::types::CorrectionType::FactMapping,
                corrected_decision: Some("d2".to_string()),
                governing_section: Some("§92e".to_string()),
                expert: "hleb".to_string(),
                note: None,
            })
            .expect("append correction");
        assert!(!c.id.is_empty());
        assert_eq!(c.episode_id, ep.id);
        assert_eq!(
            c.correction_type,
            super::super::types::CorrectionType::FactMapping
        );
    }

    fn add_item(s: &SqliteStore, content: &str, ts: TrustState, score: i64) -> StoredStrategyItem {
        s.add_strategy_item(StrategyItemInput {
            title: content.to_string(),
            description: "d".to_string(),
            content: content.to_string(),
            tags: vec!["dph".to_string()],
            trust_state: ts,
            provenance: Provenance {
                episode_id: None,
                correction_id: None,
                source: "test".to_string(),
            },
            helpful: score,
            harmful: 0,
            gate: None,
            approved_by: None,
            approved_at: None,
        })
        .expect("add item")
    }

    // store: provisional NOT retrieved; trusted + tag-match IS.
    #[test]
    fn provisional_not_retrieved_trusted_tag_match_is() {
        let s = store();
        let item = add_item(&s, "lesson", TrustState::Provisional, 0);
        assert_eq!(
            s.trusted_by_tags(&["dph".to_string()], &[], 1)
                .expect("retrieve")
                .len(),
            0,
            "provisional must be blocked"
        );
        s.update_strategy_item(
            &item.id,
            StrategyItemPatch {
                trust_state: Some(TrustState::Trusted),
                approved_by: Some("hleb".to_string()),
                approved_at: Some("2026-06-06".to_string()),
                ..Default::default()
            },
        )
        .expect("promote");
        let got = s
            .trusted_by_tags(&["dph".to_string()], &[], 1)
            .expect("retrieve");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].content, "lesson");
        // tag mismatch → blocked
        assert_eq!(
            s.trusted_by_tags(&["unrelated".to_string()], &[], 1)
                .expect("retrieve")
                .len(),
            0
        );
    }

    // store: retrieval capped at k (k≈1).
    #[test]
    fn retrieval_capped_at_k() {
        let s = store();
        for n in ["a", "b", "c"] {
            add_item(&s, n, TrustState::Trusted, 1);
        }
        assert_eq!(
            s.trusted_by_tags(&["dph".to_string()], &[], 1)
                .expect("retrieve")
                .len(),
            1
        );
    }

    // store: budget persistence (calls + spend accumulate via the store, not per-process).
    #[test]
    fn budget_persists() {
        let s = store();
        assert_eq!(s.budget_get().expect("get"), (0, 0.0));
        s.budget_charge_call().expect("charge");
        s.budget_charge_call().expect("charge");
        s.budget_record_cost(0.25).expect("record");
        s.budget_record_cost(-1.0).expect("record"); // ignored
        s.budget_record_cost(f64::NAN).expect("record"); // ignored
        let (calls, spent) = s.budget_get().expect("get");
        assert_eq!(calls, 2);
        assert!((spent - 0.25).abs() < 1e-9);
    }

    // store: list filter by trust state.
    #[test]
    fn list_filters_by_trust_state() {
        let s = store();
        add_item(&s, "p", TrustState::Provisional, 0);
        add_item(&s, "t", TrustState::Trusted, 0);
        assert_eq!(s.list_strategy_items(None).expect("list").len(), 2);
        assert_eq!(
            s.list_strategy_items(Some(TrustState::Provisional))
                .expect("list")
                .len(),
            1
        );
        assert_eq!(
            s.list_strategy_items(Some(TrustState::Trusted))
                .expect("list")
                .len(),
            1
        );
    }
}
