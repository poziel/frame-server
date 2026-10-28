#[cfg(feature = "tauri")]
fn main() {
    tauri_plugin::Builder::new(&["profiles", "spawn", "write", "resize", "close"]).build();
}
#[cfg(not(feature = "tauri"))]
fn main() {}
