use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};
use tokio::sync::mpsc;

pub type Result<T> = std::result::Result<T, AgentError>;
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("{0}")]
    Message(String),
    #[error("Unsupported capability: {0}")]
    Unsupported(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}
impl AgentError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Codex,
    Claude,
    Gemini,
    Copilot,
    Opencode,
}
impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_value(self).unwrap().as_str().unwrap()
        )
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorConfig {
    /// Restrict model tools to host-supplied MCP servers. Never user-overridable in an embedded host.
    #[serde(default)]
    pub app_tools_only: bool,
    /// Exact unqualified tool names exposed by each owned MCP server.
    #[serde(default)]
    pub app_tool_names: Vec<String>,
    /// Host-owned, ephemeral stdio servers; never persisted in provider user configuration.
    #[serde(default)]
    pub mcp_servers: BTreeMap<String, StdioMcpServer>,
    pub executable: Option<PathBuf>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    pub endpoint: Option<String>,
    pub bearer_token: Option<String>,
    pub python: Option<PathBuf>,
    pub helper: Option<PathBuf>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StdioMcpServer {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl ConnectorConfig {
    pub fn app_mcp_tool_filter(&self) -> Vec<String> {
        self.mcp_servers
            .keys()
            .flat_map(|server| {
                self.app_tool_names
                    .iter()
                    .map(move |tool| format!("mcp:{server}-{tool}"))
            })
            .collect()
    }

    pub fn mcp_configuration(&self, provider: Provider) -> Value {
        use serde_json::json;
        if provider == Provider::Gemini {
            return Value::Array(self.mcp_servers.iter().map(|(name,s)| json!({
                "name":name,"command":s.command,"args":s.args,
                "env":s.env.iter().map(|(name,value)| json!({"name":name,"value":value})).collect::<Vec<_>>()
            })).collect());
        }
        Value::Object(self.mcp_servers.iter().map(|(name,s)| {
            let value = match provider {
                Provider::Opencode => json!({"type":"local","command":std::iter::once(s.command.clone()).chain(s.args.clone()).collect::<Vec<_>>(),"environment":s.env,"enabled":true}),
                Provider::Claude | Provider::Copilot => json!({"type":"stdio","command":s.command,"args":s.args,"env":s.env}),
                _ => json!({"command":s.command,"args":s.args,"env":s.env}),
            };
            (name.clone(),value)
        }).collect())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub resume: bool,
    pub list_sessions: bool,
    pub approvals: bool,
    pub questions: bool,
    pub tools: bool,
    pub artifacts: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionOptions {
    pub cwd: String,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub resume_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub title: Option<String>,
    pub cwd: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub text: String,
    pub turn_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<PromptImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptImage {
    pub name: String,
    pub path: String,
    pub data: String,
}

impl Prompt {
    pub fn parts(&self, provider: Provider) -> Vec<Value> {
        use serde_json::json;
        let mut parts = vec![json!({"type":"text","text":self.text})];
        parts.extend(self.images.iter().map(|image| match provider {
            Provider::Codex => json!({"type":"localImage","path":image.path}),
            Provider::Gemini => json!({"type":"image","data":image.data,"mimeType":"image/png"}),
            Provider::Claude => json!({"type":"image","source":{"type":"base64","media_type":"image/png","data":image.data}}),
            Provider::Opencode => json!({"type":"file","mime":"image/png","url":format!("data:image/png;base64,{}",image.data),"filename":image.name}),
            Provider::Copilot => json!({"type":"file","path":image.path,"displayName":image.name}),
        }));
        parts
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub id: String,
    pub text: String,
    pub choices: Vec<Choice>,
    pub multiple: bool,
    pub allow_custom: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub question_id: String,
    pub choice_ids: Vec<String>,
    pub custom_text: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum InteractionResponse {
    Approval { decision: String },
    Answers { answers: Vec<Answer> },
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum EventKind {
    Message {
        id: String,
        text: String,
        phase: String,
        delta: bool,
    },
    Thinking {
        id: String,
        text: String,
        delta: bool,
        finished: bool,
    },
    Activity {
        id: String,
        label: String,
        text: String,
        output: String,
        status: String,
    },
    Approval {
        request_id: String,
        text: String,
        detail: Option<String>,
    },
    Questions {
        request_id: String,
        text: String,
        questions: Vec<Question>,
    },
    Artifact {
        id: String,
        name: String,
        url: Option<String>,
        path: Option<String>,
        mime_type: Option<String>,
    },
    Status {
        message: String,
    },
    Metadata {
        name: String,
        data: Value,
    },
    Completed {
        status: String,
        error: Option<String>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    pub session_id: String,
    pub turn_id: String,
    #[serde(flatten)]
    pub event: EventKind,
}
pub type EventSink = mpsc::UnboundedSender<AgentEvent>;
pub fn emit(sink: &EventSink, session: &str, turn: &str, event: EventKind) {
    let _ = sink.send(AgentEvent {
        session_id: session.into(),
        turn_id: turn.into(),
        event,
    });
}

pub fn validate_answers(questions: &[Question], answers: &[Answer]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for answer in answers {
        if !seen.insert(&answer.question_id) {
            return Err(AgentError::msg("Duplicate question answer"));
        }
        let question = questions
            .iter()
            .find(|q| q.id == answer.question_id)
            .ok_or_else(|| AgentError::msg("Unknown question"))?;
        let selected: std::collections::HashSet<_> = answer.choice_ids.iter().collect();
        let custom = answer.custom_text.as_deref().unwrap_or("").trim();
        if selected.len() != answer.choice_ids.len()
            || selected
                .iter()
                .any(|id| !question.choices.iter().any(|c| &c.id == *id))
            || (!custom.is_empty() && !question.allow_custom)
            || (!question.multiple && selected.len() + usize::from(!custom.is_empty()) > 1)
            || (selected.is_empty() && custom.is_empty())
        {
            return Err(AgentError::msg("Invalid question answer"));
        }
    }
    if seen.len() != questions.len() {
        return Err(AgentError::msg("Answer every required question"));
    }
    Ok(())
}

#[async_trait]
pub trait AgentConnector: Send + Sync {
    fn provider(&self) -> Provider;
    fn capabilities(&self) -> Capabilities;
    async fn create_session(&self, options: SessionOptions) -> Result<SessionInfo>;
    /// Catalog from an already connected session. Hosts must not call this to boot providers.
    async fn list_models(&self, _session: &str) -> Result<Vec<AgentModel>> {
        Err(AgentError::Unsupported("model catalog".into()))
    }
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        Err(AgentError::Unsupported("provider-wide history".into()))
    }
    async fn send(&self, session: &str, prompt: Prompt, sink: EventSink) -> Result<()>;
    async fn respond(
        &self,
        session: &str,
        request_id: &str,
        response: InteractionResponse,
    ) -> Result<()>;
    async fn cancel(&self, session: &str) -> Result<()>;
    async fn dispose(&self) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentModel {
    pub id: String,
    pub label: String,
}

pub fn model_catalog(value: &Value, id_key: &str, label_key: &str) -> Vec<AgentModel> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["hidden"] != true && m["policy"]["state"] != "disabled")
        .filter_map(|m| {
            let id = m[id_key].as_str()?.to_owned();
            Some(AgentModel {
                label: m[label_key].as_str().unwrap_or(&id).to_owned(),
                id,
            })
        })
        .collect()
}

#[cfg(test)]
mod app_policy_tests {
    use super::*;
    #[test]
    fn copilot_filter_exposes_only_exact_owned_artwork_tools() {
        let config: ConnectorConfig = serde_json::from_value(serde_json::json!({
            "appToolsOnly":true,"appToolNames":["draw","get_project_info"],
            "mcpServers":{"pixen_art":{"command":"pixen"}}
        }))
        .unwrap();
        assert_eq!(
            config.app_mcp_tool_filter(),
            vec!["mcp:pixen_art-draw", "mcp:pixen_art-get_project_info"]
        );
        assert!(ConnectorConfig::default().app_mcp_tool_filter().is_empty());
    }
}
