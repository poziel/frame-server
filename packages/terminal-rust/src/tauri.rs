//! Tauri permissions and lifecycle bridge; PTY execution belongs to Frame Server.
use crate::{Output, SpawnOptions, Terminals};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tauri::{
    ipc::Channel,
    plugin::{Builder, TauriPlugin},
    Manager, Runtime, State, Webview,
};

type Result<T> = std::result::Result<T, String>;

#[derive(Default)]
struct Bridge {
    engine: Arc<Terminals>,
    owners: Mutex<HashMap<String, String>>,
}

#[tauri::command]
async fn profiles(state: State<'_, Bridge>) -> Result<Vec<crate::Profile>> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || engine.profiles())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn spawn<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Bridge>,
    options: SpawnOptions,
    output: Channel<Output>,
) -> Result<String> {
    let owner = webview.label().to_string();
    let window = webview.window().label().to_string();
    let app = webview.app_handle().clone();
    state
        .owners
        .lock()
        .map_err(|e| e.to_string())?
        .insert(owner.clone(), window.clone());
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let id = engine.spawn(
            &owner,
            options,
            Arc::new(move |event| output.send(event).map_err(|e| e.to_string())),
        )?;
        if app.get_webview_window(&window).is_none() {
            let _ = engine.close(&owner, &id);
            return Err("Terminal window closed during startup".into());
        }
        Ok(id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn write<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Bridge>,
    id: String,
    data: String,
) -> Result<()> {
    let engine = state.engine.clone();
    let owner = webview.label().to_string();
    tauri::async_runtime::spawn_blocking(move || engine.write(&owner, &id, &data))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn resize<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Bridge>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<()> {
    let engine = state.engine.clone();
    let owner = webview.label().to_string();
    tauri::async_runtime::spawn_blocking(move || engine.resize(&owner, &id, cols, rows))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn close<R: Runtime>(
    webview: Webview<R>,
    state: State<'_, Bridge>,
    id: String,
) -> Result<()> {
    let engine = state.engine.clone();
    let owner = webview.label().to_string();
    tauri::async_runtime::spawn_blocking(move || engine.close(&owner, &id))
        .await
        .map_err(|e| e.to_string())?
}

/// Install only for trusted local content with explicit terminal permissions.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("frame-terminal")
        .invoke_handler(tauri::generate_handler![
            profiles, spawn, write, resize, close
        ])
        .setup(|app, _| {
            app.manage(Bridge::default());
            Ok(())
        })
        .on_event(|app, event| {
            let window = match event {
                tauri::RunEvent::WindowEvent {
                    label,
                    event: tauri::WindowEvent::Destroyed,
                    ..
                } => Some(label.as_str()),
                _ => None,
            };
            if window.is_some() || matches!(event, tauri::RunEvent::Exit) {
                let bridge = app.state::<Bridge>();
                let owners = bridge
                    .owners
                    .lock()
                    .map(|mut owners| {
                        let removed: Vec<_> = owners
                            .iter()
                            .filter(|(_, label)| window.is_none() || window == Some(label.as_str()))
                            .map(|(owner, _)| owner.clone())
                            .collect();
                        for owner in &removed {
                            owners.remove(owner);
                        }
                        removed
                    })
                    .unwrap_or_default();
                for owner in owners {
                    bridge.engine.close_owner(&owner);
                }
            }
        })
        .build()
}
