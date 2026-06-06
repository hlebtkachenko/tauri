mod engine;

use std::sync::Mutex;

use tauri::ipc::Channel;
use tauri::{Manager, State};
use tauri_specta::{collect_commands, Builder};

use engine::classify::ask as classify_ask;
use engine::gold::LiveScorer;
use engine::learn;
use engine::store::{SqliteStore, Store};
use engine::topics::load_topics;
use engine::types::{
    CorrectionInput, EngineError, Episode, Outcome, StoredStrategyItem, Topic, TrustState,
};

/// The store, managed as Tauri state behind a Mutex. rusqlite's `Connection` is `!Sync` and
/// the `MutexGuard` is `!Send`, so commands must NEVER hold this lock across an `.await`
/// (see `ask`). The Mutex makes the !Sync store shareable; the locking discipline keeps it
/// off the async hot path.
type StoreState = Mutex<SqliteStore>;

/// The IPC-friendly result of submitting a correction (the `SubmitOutcome` from learn.rs is
/// not Serialize; this is its wire shape). `routed` is "gate" (a fact_mapping lesson went
/// through the verification gate) or "human" (a rule/vocabulary correction was queued).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SubmitResult {
    pub routed: String,
    pub correction: engine::types::Correction,
    pub item: Option<StoredStrategyItem>,
}

// Example typed command. Frontend calls it via the generated, fully-typed
// `commands.greet(...)` in src/lib/bindings.ts — no stringly-typed `invoke`.
#[tauri::command]
#[specta::specta]
fn greet(name: String) -> String {
    format!("Hello, {name}! You've been greeted from Rust.")
}

/// Ask a regulated question. Fail-closed: any error becomes an `Outcome::Abstain` — never an
/// `Err` to the UI, never a panic. Emits a progress string per stage via `on_progress`.
///
/// Concurrency: the store lock is taken ONLY inside the two synchronous closures
/// (`retrieve`, `record`), each of which locks → does its blocking rusqlite work → drops the
/// guard at the end of the closure. No `MutexGuard` is ever held across an `.await`; the
/// async route/extract/solve work in between holds no lock.
#[tauri::command]
#[specta::specta]
async fn ask(
    state: State<'_, StoreState>,
    question: String,
    as_of_date: Option<String>,
    on_progress: Channel<String>,
) -> Result<Outcome, EngineError> {
    // Tauri requires async commands with reference inputs to return `Result`, but this body
    // provably only ever returns `Ok(...)`: `classify_ask` returns a plain `Outcome` (it maps
    // EVERY internal error — retrieve/lock/extract/solve/timeout — to `Outcome::Abstain`), and
    // nothing here uses `?`. So the UI can only ever receive an answer or an abstain, never an
    // `Err` or a thrown exception. The `Ok` wrapper is a type formality, not a failure path.
    let as_of = as_of_date.unwrap_or_default();

    let retrieve =
        |tags: &[String], k: usize| -> Result<Vec<engine::types::StrategyItem>, EngineError> {
            let store = state.lock().map_err(|_| poisoned())?;
            store.trusted_by_tags(tags, &[], k)
            // guard dropped here at end of closure scope — never crosses an await
        };
    let record = |episode: engine::types::EpisodeInput| {
        // Best effort: a failed episode append must not change the answer.
        if let Ok(store) = state.lock() {
            let _ = store.append_episode(episode);
        }
        // guard dropped here at end of closure scope
    };
    let on_progress_fn = |stage: &str| {
        let _ = on_progress.send(stage.to_string());
    };

    let outcome = classify_ask(&question, &as_of, &on_progress_fn, retrieve, record).await;
    Ok(outcome)
}

/// Submit an expert correction (uses the `LiveScorer` for the verification gate). A
/// `fact_mapping` lesson is distilled → gated → stored provisional; a rule/vocabulary
/// correction is queued for a human. The store lock is held only for this synchronous call;
/// the LiveScorer builds its own runtime internally and holds no store lock.
#[tauri::command]
#[specta::specta]
fn submit_correction(
    state: State<'_, StoreState>,
    input: CorrectionInput,
) -> Result<SubmitResult, EngineError> {
    let as_of = "".to_string();
    let scorer = LiveScorer::new(as_of);
    let store = state.lock().map_err(|_| poisoned())?;
    let out = learn::submit_correction(&*store, input, &scorer)?;
    Ok(SubmitResult {
        routed: match out.routed {
            learn::Routed::Gate => "gate".to_string(),
            learn::Routed::Human => "human".to_string(),
        },
        correction: out.correction,
        item: out.item,
    })
}

/// Human approval (provisional → trusted). Refused unless the verification gate passed.
#[tauri::command]
#[specta::specta]
fn approve(
    state: State<'_, StoreState>,
    id: String,
    approver: String,
) -> Result<StoredStrategyItem, EngineError> {
    let store = state.lock().map_err(|_| poisoned())?;
    learn::approve(&*store, &id, &approver)
}

/// Every registered accounting topic (loaded from the bundled rules modules).
#[tauri::command]
#[specta::specta]
fn list_topics() -> Vec<Topic> {
    load_topics().unwrap_or_default()
}

/// The most recent `n` episodes (the raw material corrections attach to).
#[tauri::command]
#[specta::specta]
fn recent_episodes(state: State<'_, StoreState>, n: u32) -> Vec<Episode> {
    match state.lock() {
        Ok(store) => store.recent_episodes(n as usize).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Strategy items, optionally filtered by trust state ("provisional" | "trusted"). An
/// unrecognized filter string yields an empty list (fail-closed).
#[tauri::command]
#[specta::specta]
fn list_strategy_items(
    state: State<'_, StoreState>,
    trust_state: Option<String>,
) -> Vec<StoredStrategyItem> {
    let filter = match trust_state {
        Some(s) => match TrustState::from_str(&s) {
            Ok(ts) => Some(ts),
            Err(_) => return Vec::new(),
        },
        None => None,
    };
    match state.lock() {
        Ok(store) => store.list_strategy_items(filter).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn poisoned() -> EngineError {
    EngineError::Store("store mutex poisoned".to_string())
}

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        greet,
        ask,
        submit_correction,
        approve,
        list_topics,
        recent_episodes,
        list_strategy_items
    ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();

    // Regenerate the TypeScript bindings on every dev run.
    #[cfg(debug_assertions)]
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/lib/bindings.ts",
        )
        .expect("failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        // Battery: avoid the white startup flash. The window is created hidden
        // (visible: false in tauri.conf.json); show it once the page has loaded.
        .on_page_load(|webview, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Finished {
                let _ = webview.window().show();
            }
        })
        .setup(move |app| {
            builder.mount_events(app);
            // Open (or create) the store at the app data dir; manage it as shared state.
            let data_dir = app.path().app_data_dir().expect("resolve app data dir");
            std::fs::create_dir_all(&data_dir).expect("create app data dir");
            let db_path = data_dir.join("asmara.sqlite3");
            let store = SqliteStore::open(&db_path.to_string_lossy()).expect("open sqlite store");
            app.manage(Mutex::new(store));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    // Generates src/lib/bindings.ts without launching the app (used by CI).
    // Run with: cargo test --manifest-path src-tauri/Cargo.toml
    #[test]
    fn export_bindings() {
        super::specta_builder()
            .export(
                specta_typescript::Typescript::default(),
                "../src/lib/bindings.ts",
            )
            .expect("failed to export typescript bindings");
    }
}
