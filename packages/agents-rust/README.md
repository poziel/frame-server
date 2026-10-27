<p align="center">
  <img src="https://raw.githubusercontent.com/poziel/frame/main/assets/frame.svg" alt="Frame logo" width="72" height="72">
</p>

<h1 align="center">frame-agent-connectors</h1>

<p align="center">Connect local AI agent tools to your Rust application.</p>

<p align="center">
  <a href="https://crates.io/crates/frame-agent-connectors"><img src="https://img.shields.io/crates/v/frame-agent-connectors?style=flat" alt="Version on crates.io"></a>
  <a href="https://docs.rs/frame-agent-connectors"><img src="https://img.shields.io/docsrs/frame-agent-connectors?style=flat" alt="API documentation"></a>
  <a href="https://github.com/poziel/frame-server/blob/main/LICENSE"><img src="https://img.shields.io/github/license/poziel/frame-server?style=flat" alt="MIT license"></a>
</p>

Embed Codex, Claude Agent SDK, Gemini CLI, GitHub Copilot SDK, and OpenCode
connectors through a shared Rust runtime, or run the optional
`frame-agent-server` JSONL executable. Provider connections and events are
independent of any UI framework.

Your application supplies credentials, provider tools, authorization, and
process lifecycle. Frame applications can connect this runtime to their
[chat module](https://github.com/poziel/frame/tree/main/packages/core/src/modules/chat)
through a host-owned transport.

## Install

Add the public crates.io package to your Rust application:

```toml
[dependencies]
frame-agent-connectors = "0.1.1"
```

No registry account or download token is required. Provider authentication is
configured separately by your application.

## Embed the runtime

For the following example, also add Tokio:

```toml
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use frame_agent_connectors::runtime::{AgentRuntime, RuntimeConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = AgentRuntime::new(RuntimeConfig::default()).await?;
    println!("{}", runtime.capabilities());
    runtime.dispose().await?;
    Ok(())
}
```

This prints connector capabilities without starting a provider or authenticating.
Creating a connector or starting the host executable does not start a provider.
Explicit session creation, provider-wide history requests, and subsequent
operations can start or contact the selected provider.

## Run the optional host executable

To install the executable from crates.io with Rust/Cargo:

```sh
cargo install frame-agent-connectors --version 0.1.1 --locked
frame-agent-server --help
```

Applications embedding the library do not need the executable. Hosts using it
must distribute the compiled binary, any selected provider executables, and
Claude's helper and Python environment explicitly.

To run from a repository checkout instead:

```sh
cargo run -p frame-agent-connectors --bin frame-agent-server -- --help
```

## Download a source archive

The [v0.1.1 GitHub Release](https://github.com/poziel/frame-server/releases/tag/v0.1.1)
also provides [frame-agent-connectors-0.1.1.crate](https://github.com/poziel/frame-server/releases/download/v0.1.1/frame-agent-connectors-0.1.1.crate).
Download it into your application's directory and extract it:

```sh
tar -xf frame-agent-connectors-0.1.1.crate
```

```toml
[dependencies]
frame-agent-connectors = { path = "frame-agent-connectors-0.1.1" }
```

This is a source package, not a prebuilt executable. Cargo resolves third-party
dependencies when building. It also contains the Claude Python helper and its
requirements file.

## Configure providers

| Provider   | Required host setup                                                                            | Transport                                                  | Resume / provider-wide history                                                 |
| ---------- | ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `codex`    | Installed `codex` CLI with app-server support and host-managed credentials                     | `codex app-server`, bidirectional JSONL                    | Both                                                                           |
| `claude`   | Python 3.10+, requirements below, and Claude SDK authentication configured by the host         | Python bridge using the official Claude Agent SDK          | Both; SDK history API must be available                                        |
| `gemini`   | Installed Gemini CLI with ACP support and host-managed credentials                             | `gemini --acp`, ACP v1                                     | Resume only when the server advertises `loadSession`; no provider-wide listing |
| `copilot`  | Installed `copilot` CLI compatible with the pinned official Rust SDK; host-managed credentials | Official Rust SDK 1.0.26 using explicit CLI path and stdio | Both                                                                           |
| `opencode` | An already running `opencode serve` instance                                                   | HTTP and `/event` SSE                                      | Both                                                                           |

Claude requires an explicit Python package installation. From the repository root:

```powershell
python -m venv packages/agents-rust/.venv
packages/agents-rust/.venv/Scripts/python.exe -m pip install -r packages/agents-rust/python/requirements.txt
```

The requirement is pinned in [python/requirements.txt](https://github.com/poziel/frame-server/blob/main/packages/agents-rust/python/requirements.txt). On Unix, use `.venv/bin/python`. Set `connectors.claude.python` to that interpreter. Distribute [frame_claude_agent.py](https://github.com/poziel/frame-server/blob/main/packages/agents-rust/python/frame_claude_agent.py) alongside your application and set `helper` to its absolute path; the default helper path points into this source checkout at build time.

Create a host-owned configuration file and pass `--config PATH`:

```json
{
  "applicationId": "my-app",
  "historyPath": "D:/AppData/my-app/agent-sessions.json",
  "allowedRoots": ["D:/Projects"],
  "allowAllHistory": false,
  "connectors": {
    "claude": {
      "python": "D:/Tools/agent-python/Scripts/python.exe",
      "helper": "D:/Apps/my-app/frame_claude_agent.py"
    },
    "opencode": {
      "endpoint": "http://127.0.0.1:4096"
    }
  }
}
```

Replace the example paths with existing host paths. `allowedRoots` bounds canonical session working directories. `applicationId` pins the application identity accepted by the runtime. The optional ownership registry records sessions created or resumed through this runtime. Without `historyPath`, that registry lasts only for the current process. Application history is the default; provider-wide history and adopting unowned provider sessions require `allowAllHistory: true`.

Connector settings are `executable`, `args`, `env`, `endpoint`, `bearerToken`, `python`, and `helper`. Use only settings supported by the selected connector:

Hosts may also supply `mcpServers`, a map of stdio server names to
`{ "command": "absolute executable", "args": [], "env": {} }`. These are forwarded
to Codex thread configuration, Claude SDK session options, Gemini ACP session
creation/loading, Copilot SDK session configuration, and OpenCode's dynamic MCP
endpoint. OpenCode registration is scoped to the session working directory;
use an isolated host working directory and unique names for document-bound tools.
Disposal disconnects those named OpenCode MCP servers while leaving the external
server running. No provider's persistent user configuration is written.

- Codex and Gemini use `executable` or the named CLI on `PATH`. Nonempty `args` replace their default arguments.
- Claude uses `python`, then `executable`, then `python` on `PATH`. Nonempty `args` replace the helper invocation.
- Copilot uses `executable` or `copilot` on `PATH`; `args` are additional CLI arguments. The connector selects an explicit program and does not ask the SDK to download or resolve a bundled CLI.
- Child-process connectors forward `env`. Their processes also inherit the host environment.
- OpenCode uses `endpoint`, defaulting to `http://127.0.0.1:4096`. It does not launch or stop the external server. `bearerToken` supports a host's bearer-authenticated endpoint. OpenCode's own Basic authentication can be supplied explicitly through `env.OPENCODE_SERVER_PASSWORD` and optional `env.OPENCODE_SERVER_USERNAME` (default `opencode`). Bearer authentication takes precedence. These entries configure HTTP authentication; they do not spawn a process or modify environment variables.

## Host protocol

Connect the binary's stdin/stdout to a host transport. Stdout carries one JSON object per line. Requests have an `id`, `method`, and `params`; responses carry the same `id` and either `result` or `error.message`. IDs allow concurrent requests, including approval replies and cancellation while a turn is streaming.

A session creation request:

```json
{"id":1,"method":"session/create","params":{"provider":"codex","applicationId":"my-app","cwd":"D:/Projects/example"}}
```

The result includes a runtime `sessionId`, the durable `providerSessionId`, provider name, and capabilities. Use the runtime handle for turn and interaction methods; use the durable provider ID as `resumeId` when creating a resumed session.

| Method                                 | Parameters                                                                       |
| -------------------------------------- | -------------------------------------------------------------------------------- |
| `initialize`, `connector/capabilities` | `{}`                                                                             |
| `session/create`                       | `provider`, `applicationId`, `cwd`; optional `model`, `systemPrompt`, `resumeId` |
| `session/list`                         | `provider`, `applicationId`; optional `scope: "all"`                             |
| `turn/send`                            | `sessionId`, `turnId`, `text`; optional `images`                                 |
| `turn/cancel`                          | `sessionId`; optional `turnId`                                                   |
| `interaction/respond`                  | `sessionId`, `turnId`, `requestId`, `response`                                   |
| `session/close`                        | `sessionId`                                                                      |
| `models/list`                          | `sessionId` of an already connected session                                      |
| `shutdown`                             | `{}`                                                                             |

Notifications have `method: "event"` and `params` containing `sessionId`, `turnId`, and a typed event. Message phases are `intermediate` or `final`. Events include text, thinking, activities, approvals, questions, artifacts, status, provider metadata, and terminal completion. Subscribe before sending a prompt. A turn's request remains pending while events stream; handle events and responses independently.

Approval responses are `{ "type": "approval", "decision": "allow" }` or `"deny"`. Question responses are `{ "type": "answers", "answers": [{ "questionId": "...", "choiceIds": ["..."], "customText": "..." }] }`; omit custom text for an offered choice. `{ "type": "cancel" }` cancels an interaction. Offered choices and custom-answer permissions are validated before forwarding answers.

Codex granular permission approval grants exactly the requested permission profile for the current turn; denial grants an empty profile. No connector automatically approves an interaction. Codex secret questions require a separate host UI and are rejected by this adapter.

## Frame integration and boundaries

Use `createJsonLineAgentConnection` with a host-provided `AgentLineTransport`, then `createAgentChatSession` with that connection and session options. These helpers do not spawn native processes or register shell views. The resulting chat can be embedded or registered explicitly using the existing chat view API.

The host owns process startup, credentials, the requested working directory, and artifact access. Local artifact paths remain metadata until the application's `resolveArtifact` authorizes and exposes them. Copilot file-edit artifacts and OpenCode file parts preserve paths; they do not turn native paths into browser `file:` URLs. Raw provider metadata is retained for details the common chat model does not represent.

One active turn is allowed per runtime session. Cancellation stops the active turn; session close removes the runtime handle while retaining provider history. Runtime disposal stops owned connections/processes and settles outstanding work. The OpenCode server remains externally owned.

Native session history is authoritative. The chat adapter rejects local clear/rewind/history replacement; create a new session for a new conversation. Text and image prompts are supported. Hosts supply validated PNG images as `images: [{ name, path, data }]`, where `path` is an app-owned immutable snapshot and `data` is base64 PNG. Codex/Copilot use snapshot paths; Claude/Gemini/OpenCode receive image content. Hosts own decoding, limits, file access and snapshot lifetime. Text documents can be incorporated into the prompt. This does not enable arbitrary filesystem access from chat attachments. Gemini does not expose questions or artifacts through this adapter. Codex and Claude do not expose normalized artifacts. Capabilities describe connector support, not model availability or credential validity.

## Verification

From the repository root:

```powershell
cargo test -p frame-agent-connectors
python -m unittest discover -s packages/agents-rust/python -p 'test_*.py'
```

Rust process-wire tests use Python from `FRAME_TEST_PYTHON` or `python` on `PATH`. They run strict offline fake peers. HTTP tests use loopback mock servers; Copilot callback tests use the SDK types without starting its runtime. The Python helper suite supplies a fake SDK and does not require installing Claude's SDK.

OpenCode's installed 1.19.3 server schema was inspected locally without prompts. Automated checks cover protocol framing, event mapping, scoped interactions, cancellation, and lifecycle behavior. They do not establish successful authentication, model access, real inference, or compatibility with every installed CLI version. Provider setup and a deliberately authorized live prompt remain host acceptance work.

### Embedded app tool policy

Hosts can force `ConnectorConfig.app_tools_only` (`appToolsOnly` in JSON) and
supply `appToolNames` alongside their owned `mcpServers`. Codex disables ambient
integrations and external execution features with a read-only execution sandbox;
Claude disables built-ins, inherited settings and external tool permissions;
Copilot uses exact source-qualified MCP tool names; OpenCode requires confirmed
deny-by-default session permissions. These restrictions do not replace an OS
sandbox around an untrusted provider executable. Model inference/authentication
still contact the provider. Gemini ACP rejects app-only sessions before starting
its CLI because this connector has no verified built-in tool ceiling. Ordinary
Frame Server hosts retain the existing behavior unless they opt into this policy.

[API documentation](https://docs.rs/frame-agent-connectors) ·
[Frame Server](https://github.com/poziel/frame-server) ·
[Contributing](https://github.com/poziel/frame-server/blob/main/CONTRIBUTING.md) ·
[Security](https://github.com/poziel/frame-server/blob/main/SECURITY.md) ·
[MIT license](https://github.com/poziel/frame-server/blob/main/LICENSE)
