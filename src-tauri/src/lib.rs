mod engine;

use tauri_specta::{collect_commands, Builder};

// Example typed command. Frontend calls it via the generated, fully-typed
// `commands.greet(...)` in src/lib/bindings.ts — no stringly-typed `invoke`.
#[tauri::command]
#[specta::specta]
fn greet(name: String) -> String {
    format!("Hello, {name}! You've been greeted from Rust.")
}

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![greet])
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
