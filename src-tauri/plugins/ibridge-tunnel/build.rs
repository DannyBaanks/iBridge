const COMMANDS: &[&str] = &["start", "status", "stop"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .ios_path("ios")
        .try_build()
        .expect("failed to build iBridge tunnel plugin");
}
