mod commands;
mod state;

pub use commands::*;
pub use state::TtsState;

use tauri::Manager;

pub fn init() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("supertonic")
        .setup(|app, _api| {
            app.manage(TtsState::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::synthesize,
            commands::load_model,
            commands::list_voices,
            commands::select_voice,
            commands::get_status,
        ])
        .build()
}
