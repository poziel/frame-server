//! Optional Tauri transport. Applications retain runtime configuration and request policy.
use crate::{runtime::AgentRuntime, AgentEvent, EventSink};
pub use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tauri::{
    ipc::Channel,
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, Runtime, Webview,
};

/// Native identity; never populated from frontend request arguments.
#[derive(Clone)]
pub struct AgentContext<R: Runtime> {
    pub app: AppHandle<R>,
    pub webview: String,
    pub window: String,
}
impl<R: Runtime> AgentContext<R> {
    fn from_webview(webview: &Webview<R>) -> Self {
        Self {
            app: webview.app_handle().clone(),
            webview: webview.label().into(),
            window: webview.window().label().into(),
        }
    }
}
/// Application policy and ownership boundary behind the reusable IPC transport.
/// Implementations must reject IDs owned by another webview and stop accepting
/// new work before disposing a connection. Only register for trusted local content.
#[async_trait]
pub trait AgentHost<R: Runtime>: Send + Sync + 'static {
    async fn request(
        &self,
        context: AgentContext<R>,
        runtime_id: String,
        method: String,
        params: Value,
    ) -> Result<Value, String>;
    async fn disconnect(&self, context: AgentContext<R>, runtime_id: String) -> Result<(), String>;
    async fn close_window(&self, app: AppHandle<R>, window: String);
}
struct HostState<R: Runtime>(Arc<dyn AgentHost<R>>);

#[tauri::command]
async fn request<R: Runtime>(
    webview: Webview<R>,
    state: tauri::State<'_, HostState<R>>,
    runtime_id: String,
    method: String,
    params: Value,
) -> Result<Value, String> {
    state
        .0
        .request(
            AgentContext::from_webview(&webview),
            runtime_id,
            method,
            params,
        )
        .await
}
#[tauri::command]
async fn disconnect<R: Runtime>(
    webview: Webview<R>,
    state: tauri::State<'_, HostState<R>>,
    runtime_id: String,
) -> Result<(), String> {
    state
        .0
        .disconnect(AgentContext::from_webview(&webview), runtime_id)
        .await
}
/// Install one native transport; installation starts no runtime or provider.
pub fn init<R: Runtime>(host: impl AgentHost<R>) -> TauriPlugin<R> {
    let host: Arc<dyn AgentHost<R>> = Arc::new(host);
    let setup_host = host.clone();
    Builder::new("frame-agent-connectors")
        .invoke_handler(tauri::generate_handler![request, disconnect])
        .setup(move |app, _| {
            if !app.manage(HostState(setup_host.clone())) {
                return Err("Frame agent transport is already installed".into());
            }
            Ok(())
        })
        .on_event(move |app, event| {
            if let tauri::RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::Destroyed,
                ..
            } = event
            {
                let host = host.clone();
                let app = app.clone();
                let label = label.clone();
                tauri::async_runtime::spawn(async move {
                    host.close_window(app, label).await;
                });
            }
        })
        .build()
}
/// Stream runtime events to a Tauri channel. Receiver loss disposes the runtime.
/// The caller owns normal disposal and must keep an application scope alive
/// until outstanding requests finish. No provider starts merely by wiring events.
pub fn event_sink(runtime: Arc<AgentRuntime>, events: Channel<AgentEvent>) -> EventSink {
    let (sink, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = receiver.recv().await {
            if events.send(event).is_err() {
                break;
            }
        }
        let _ = runtime.dispose().await;
    });
    sink
}

#[cfg(all(test, feature = "tauri-tests"))]
mod tests {
    use super::*;
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};

    #[derive(Default)]
    struct TestHost(Arc<std::sync::Mutex<Vec<String>>>);
    #[async_trait]
    impl AgentHost<MockRuntime> for TestHost {
        async fn request(
            &self,
            context: AgentContext<MockRuntime>,
            runtime_id: String,
            method: String,
            params: Value,
        ) -> Result<Value, String> {
            if context.webview != "owner" || runtime_id != "owned" {
                return Err("Unauthorized connection".into());
            }
            Ok(
                serde_json::json!({"owner":context.webview, "window":context.window, "method":method, "params":params}),
            )
        }
        async fn disconnect(
            &self,
            context: AgentContext<MockRuntime>,
            runtime_id: String,
        ) -> Result<(), String> {
            if context.webview != "owner" || runtime_id != "owned" {
                return Err("Unauthorized connection".into());
            }
            Ok(())
        }
        async fn close_window(&self, _: AppHandle<MockRuntime>, window: String) {
            self.0.lock().unwrap().push(window);
        }
    }

    #[tokio::test]
    async fn native_context_and_policy_are_preserved() {
        let app = mock_builder()
            .plugin(init(TestHost::default()))
            .build(mock_context(noop_assets()))
            .unwrap();
        let owner = tauri::WebviewWindowBuilder::new(&app, "owner", Default::default())
            .build()
            .unwrap();
        let other = tauri::WebviewWindowBuilder::new(&app, "other", Default::default())
            .build()
            .unwrap();
        let result = request(
            owner.as_ref().clone(),
            app.state(),
            "owned".into(),
            "models/list".into(),
            serde_json::json!({"key": 7}),
        )
        .await
        .unwrap();
        assert_eq!(result["owner"], "owner");
        assert_eq!(result["window"], "owner");
        assert_eq!(result["method"], "models/list");
        assert_eq!(result["params"]["key"], 7);
        assert!(request(
            other.as_ref().clone(),
            app.state(),
            "owned".into(),
            "models/list".into(),
            Value::Null
        )
        .await
        .is_err());
        assert!(
            disconnect(other.as_ref().clone(), app.state(), "owned".into())
                .await
                .is_err()
        );
        disconnect(owner.as_ref().clone(), app.state(), "owned".into())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn disconnected_event_channel_disposes_runtime() {
        let runtime = Arc::new(
            AgentRuntime::with_connectors(Default::default(), Default::default())
                .await
                .unwrap(),
        );
        let events = Channel::new(|_| Err(std::io::Error::other("receiver closed").into()));
        let sink = event_sink(runtime.clone(), events);
        sink.send(AgentEvent {
            session_id: "session".into(),
            turn_id: "turn".into(),
            event: crate::EventKind::Status {
                message: "test".into(),
            },
        })
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if runtime
                    .list_sessions(crate::Provider::Codex, "test", false)
                    .await
                    .err()
                    .map(|error| error.to_string())
                    .unwrap_or_default()
                    .to_string()
                    .contains("disposed")
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
}
