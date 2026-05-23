const COMMANDS: &[&str] = &[
    "synthesize",
    "load_model",
    "list_voices",
    "select_voice",
    "get_status",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
