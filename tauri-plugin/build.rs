const COMMANDS: &[&str] = &[
    "synthesize",
    "synthesize_stream",
    "load_model",
    "list_voices",
    "select_voice",
    "get_status",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
