use frame_agent_connectors::{ConnectorConfig, Provider};
use serde_json::json;

#[test]
fn host_mcp_configuration_maps_to_each_native_protocol() {
    let config: ConnectorConfig = serde_json::from_value(json!({"mcpServers":{"art":{
        "command":"/apps/pixen", "args":["--ai-mcp-proxy"], "env":{"ART_TOKEN":"test"}
    }}}))
    .unwrap();
    assert_eq!(
        config.mcp_configuration(Provider::Codex),
        json!({"art":{
            "command":"/apps/pixen","args":["--ai-mcp-proxy"],"env":{"ART_TOKEN":"test"}
        }})
    );
    assert_eq!(
        config.mcp_configuration(Provider::Claude),
        json!({"art":{
            "type":"stdio","command":"/apps/pixen","args":["--ai-mcp-proxy"],"env":{"ART_TOKEN":"test"}
        }})
    );
    assert_eq!(
        config.mcp_configuration(Provider::Gemini),
        json!([{
            "name":"art","command":"/apps/pixen","args":["--ai-mcp-proxy"],"env":[{"name":"ART_TOKEN","value":"test"}]
        }])
    );
    assert_eq!(
        config.mcp_configuration(Provider::Copilot)["art"]["type"],
        json!("stdio")
    );
    assert_eq!(
        config.mcp_configuration(Provider::Opencode),
        json!({"art":{
            "type":"local","command":["/apps/pixen","--ai-mcp-proxy"],"environment":{"ART_TOKEN":"test"},"enabled":true
        }})
    );
}

#[tokio::test]
async fn native_processes_receive_host_mcp_servers() {
    use frame_agent_connectors::{
        claude::ClaudeConnector, codex::CodexConnector, gemini::GeminiConnector, AgentConnector,
        SessionOptions,
    };
    for (provider, app_only) in [
        (Provider::Codex, false),
        (Provider::Codex, true),
        (Provider::Claude, false),
        (Provider::Gemini, false),
    ] {
        let mut config: ConnectorConfig = serde_json::from_value(json!({"mcpServers":{"art":{
            "command":"/apps/pixen","args":["--ai-mcp-proxy"],"env":{"ART_TOKEN":"test"}
        }}}))
        .unwrap();
        config.app_tools_only = app_only;
        if app_only {
            config.app_tool_names = vec!["get_project_info".into(), "apply_edits".into()];
        }
        let python = std::env::var_os("FRAME_TEST_PYTHON").unwrap_or_else(|| "python".into());
        config.executable = Some(python.clone().into());
        config.python = Some(python.into());
        config.args = vec![
            "-u".into(),
            format!("{}/tests/fixtures/mcp_peer.py", env!("CARGO_MANIFEST_DIR")),
            if app_only {
                "codex-app-only".into()
            } else {
                provider.to_string()
            },
        ];
        let connector: Box<dyn AgentConnector> = match provider {
            Provider::Codex => Box::new(CodexConnector::new(config)),
            Provider::Claude => Box::new(ClaudeConnector::new(config)),
            _ => Box::new(GeminiConnector::new(config)),
        };
        let session = connector
            .create_session(SessionOptions {
                cwd: "test-workspace".into(),
                model: None,
                system_prompt: None,
                resume_id: None,
            })
            .await
            .unwrap();
        assert_eq!(session.id, "art-session");
        let models = connector.list_models(&session.id).await.unwrap();
        assert_eq!(models[0].id, "artist");
        assert_eq!(models[0].label, "Artist");
        connector.dispose().await.unwrap();
    }
}

#[tokio::test]
#[ignore = "Requires installed Codex CLI; creates a session and lists models, no inference"]
async fn installed_codex_accepts_read_only_session_and_lists_models() {
    use frame_agent_connectors::{codex::CodexConnector, AgentConnector, SessionOptions};
    let root = tempfile::tempdir().unwrap();
    let connector = CodexConnector::new(ConnectorConfig {
        app_tools_only: true,
        ..Default::default()
    });
    let session = connector
        .create_session(SessionOptions {
            cwd: root.path().to_string_lossy().into_owned(),
            model: None,
            system_prompt: None,
            resume_id: None,
        })
        .await
        .unwrap();
    let models = connector.list_models(&session.id).await.unwrap();
    assert!(!models.is_empty());
    connector.dispose().await.unwrap();
}

#[tokio::test]
async fn opencode_registers_document_mcp_and_disconnects_without_stopping_server() {
    use frame_agent_connectors::{opencode::OpenCodeConnector, AgentConnector, SessionOptions};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let peer = tokio::spawn(async move {
        for (path, response) in [
            ("mcp", json!({"art":{"status":"connected"}})),
            (
                "session",
                json!({"id":"ses_art","directory":"/art","permission":[{"permission":"*","pattern":"*","action":"deny"},{"permission":"art_*","pattern":"*","action":"allow"}]}),
            ),
            ("mcp/art/disconnect", json!(true)),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let (header_end, length) = loop {
                let mut buffer = [0; 1024];
                let count = socket.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                    assert!(headers.starts_with(&format!("post /{path}?directory=%2fart ")));
                    let length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap_or("0")
                        .parse::<usize>()
                        .unwrap();
                    break (end + 4, length);
                }
            };
            while bytes.len() < header_end + length {
                let mut buffer = [0; 1024];
                let count = socket.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
            }
            if path == "mcp" {
                let body: serde_json::Value =
                    serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
                assert_eq!(
                    body,
                    json!({"name":"art","config":{"type":"local","command":["/apps/pixen","--ai-mcp-proxy"],"environment":{},"enabled":true}})
                );
            }
            if path == "session" {
                let body: serde_json::Value =
                    serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap();
                assert_eq!(body["permission"], response["permission"]);
            }
            let body = response.to_string();
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        }
    });
    let config: ConnectorConfig = serde_json::from_value(json!({"appToolsOnly":true,"endpoint":endpoint,"mcpServers":{"art":{"command":"/apps/pixen","args":["--ai-mcp-proxy"]}}})).unwrap();
    let connector = OpenCodeConnector::new(config);
    connector
        .create_session(SessionOptions {
            cwd: "/art".into(),
            model: None,
            system_prompt: None,
            resume_id: None,
        })
        .await
        .unwrap();
    connector.dispose().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), peer)
        .await
        .unwrap()
        .unwrap();
}
