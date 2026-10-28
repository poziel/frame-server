#[cfg(feature = "tauri")]
fn main() {
    tauri_plugin::Builder::new(&["request", "disconnect"]).build();
}
#[cfg(not(feature = "tauri"))]
fn main() {}
