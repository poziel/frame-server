<p align="center">
  <img src="https://raw.githubusercontent.com/poziel/frame/main/assets/frame.svg" alt="Frame logo" width="72" height="72">
</p>

<h1 align="center">frame-terminal</h1>

<p align="center">A Rust PTY engine for application-owned terminal sessions.</p>

<p align="center">
  <a href="https://crates.io/crates/frame-terminal"><img src="https://img.shields.io/crates/v/frame-terminal?style=flat" alt="Version on crates.io"></a>
  <a href="https://docs.rs/frame-terminal"><img src="https://img.shields.io/docsrs/frame-terminal?style=flat" alt="API documentation"></a>
  <a href="https://github.com/poziel/frame-server/blob/main/LICENSE"><img src="https://img.shields.io/github/license/poziel/frame-server?style=flat" alt="MIT license"></a>
</p>

Discover installed shells and manage terminal input, output, resizing, and
cleanup from your Rust host. The engine works independently of a UI framework,
desktop runtime, or network transport.

## Install

Add the public crates.io package to your application:

```toml
[dependencies]
frame-terminal = "0.1.1"
```

No registry account or download token is required.

## List available shells

```rust
use frame_terminal::Terminals;

fn main() {
    let terminals = Terminals::default();
    for profile in terminals.profiles() {
        println!("{}: {}", profile.id, profile.path.display());
    }
}
```

`Terminals::default()` and `profiles()` do not start a shell. To create a PTY,
call `spawn(owner, options, output)`, then use `write`, `resize`, and `close`
with the same trusted owner identity. `Output` delivers bytes, exit codes, and
errors through an `OutputSink` callback.

## Own the session lifecycle

The host derives owner identities from authenticated scopes, rather than
renderer arguments. Call `close_owner` when a host scope disconnects; dropping
`Terminals` terminates its remaining shell processes.

Return an error from the output callback when its receiver disconnects.
Callbacks should be bounded, nonblocking, and must not panic. Terminal dimensions
are limited to 1–1000 characters, and a write is limited to 1 MiB.

Methods perform blocking OS work; async hosts should call them on a blocking
worker. A headless host must answer terminal cursor queries; a renderer such
as xterm normally does this. Windows sessions retain ConPTY startup and
process-handle termination behavior.

Shells inherit the host's privileges and environment. Your application owns
process authorization and OS isolation. The optional Tauri bridge lives in this crate; other host transports are not supplied.

## Download a source archive

The [v0.1.1 GitHub Release](https://github.com/poziel/frame-server/releases/tag/v0.1.1)
also includes [frame-terminal-0.1.1.crate](https://github.com/poziel/frame-server/releases/download/v0.1.1/frame-terminal-0.1.1.crate).
Download it into your application's directory, then extract it:

```sh
tar -xf frame-terminal-0.1.1.crate
```

```toml
[dependencies]
frame-terminal = { path = "frame-terminal-0.1.1" }
```

The archive contains source code; Cargo still resolves third-party dependencies
and compiles the library for your host.

## Explore and contribute

From the Frame Server repository root:

```sh
cargo run -p frame-terminal --example profiles
cargo test -p frame-terminal --locked
```

[API documentation](https://docs.rs/frame-terminal) ·
[Architecture](https://github.com/poziel/frame-server/blob/main/docs/architecture.md) ·
[Contributing](https://github.com/poziel/frame-server/blob/main/CONTRIBUTING.md) ·
[Security](https://github.com/poziel/frame-server/blob/main/SECURITY.md) ·
[MIT license](https://github.com/poziel/frame-server/blob/main/LICENSE)

## Optional embedded Tauri adapter

Enable the `tauri` feature on this crate and register
`.plugin(frame_terminal::tauri::init())` on the application's Tauri builder.
Grant `frame-terminal:default` only to trusted local webviews. The connector
derives session ownership from Tauri and closes sessions on window destruction
or application exit. It starts no shell until an explicit spawn request.

The frontend uses `createTauriTerminalBackend()` from `@frame/tauri/terminal`.
No Rust source is shipped through that npm package. Other Rust applications
continue to use `Terminals` directly without enabling the optional feature.
Until these changes are released, use the local Frame Server checkout as a
Cargo path dependency with `features = ["tauri"]`.
