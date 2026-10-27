# frame-terminal

A host-independent Rust PTY engine. It discovers installed shells and manages
sessions without depending on a desktop framework, UI, or network transport.

Construct `Terminals::default()` and call `profiles()` to inspect shells without
starting a process. Explicitly call `spawn(owner, options, output)` to create a
shell, then `write`, `resize`, and `close` with the same trusted owner identity.
`Output` delivers bytes, exit codes, and errors through an `OutputSink` callback.

The host must derive owner identities from authenticated scopes, never renderer
arguments. `close_owner` terminates sessions for a disconnected host scope;
dropping `Terminals` terminates remaining sessions. Return an error from the
output callback when the receiver disconnects. Callback processing should be
bounded and nonblocking, and must not panic. Dimensions are limited to 1–1000
characters and a write is limited to 1 MiB. Shells inherit the host's privileges
and environment; this library is not a process sandbox.

```rust
use frame_terminal::Terminals;

let terminals = Terminals::default();
for profile in terminals.profiles() {
    println!("{}: {}", profile.id, profile.path.display());
}
```

Methods perform blocking OS work; async hosts should call them on a blocking
worker. A headless host must answer terminal cursor queries; a renderer such
as xterm normally does this. The extracted engine retains ConPTY startup and
Windows process-handle termination behavior.

Frame owns framework-specific bridges. This crate provides no Tauri commands,
Electron IPC, HTTP server, or terminal JSONL executable. See the repository
[architecture](../../docs/architecture.md) for boundaries.
