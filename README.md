# Frame Server

Host-independent execution packages for [Frame](https://github.com/poziel/frame)
applications. Embed the Rust libraries in your host; Frame Server does not
require Tauri, Electron, or Frame's UI to run.

| Directory                | Cargo package            | Purpose                                                                    |
| ------------------------ | ------------------------ | -------------------------------------------------------------------------- |
| `packages/terminal-rust` | `frame-terminal`         | Shell discovery, PTYs, input/output, resize, session ownership and cleanup |
| `packages/agents-rust`   | `frame-agent-connectors` | AI agent connectors and optional `frame-agent-server` JSONL executable     |

Version: **0.1.0**. Package directories use capability-language names. Future
implementations can sit alongside them without creating language parent folders.

## Use a package

After the `v0.1.0` tag is available:

```toml
[dependencies]
frame-terminal = { git = "https://github.com/poziel/frame-server.git", tag = "v0.1.0" }
frame-agent-connectors = { git = "https://github.com/poziel/frame-server.git", tag = "v0.1.0" }
```

Choose only the packages you need. Cargo locates their manifests within this
repository. During local development use `path = "../frame-server/packages/agents-rust"`
relative to your application's Cargo manifest instead. Crates.io publication
is separate; registry availability is not implied by a GitHub release. The
[release guide](docs/releases.md#cratesio-automation) documents initial registry
setup and trusted publishing for stable tags and development builds on main.

## Build and verify

Install stable Rust with Cargo and Python 3.10+ for the offline agent fixtures.

```sh
cargo test --workspace
cargo run -p frame-terminal --example profiles
cargo run -p frame-agent-connectors --example embedded
python -m unittest discover -s packages/agents-rust/python -p 'test_*.py'
cargo package --workspace
```

These examples start no shell or provider. The optional agent executable is
started explicitly with `cargo run -p frame-agent-connectors --bin frame-agent-server -- --help`.
Provider credentials, executables, authorization, persistence, Python helper
deployment, and OS permissions belong to the consuming application.

## Integration and project information

- [Terminal package](packages/terminal-rust/README.md)
- [Agent package](packages/agents-rust/README.md)
- [Examples](examples/README.md)
- [Architecture](docs/architecture.md)
- [Releasing and consuming packages](docs/releases.md)
- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)
- [Changelog](CHANGELOG.md)

MIT licensed; see [LICENSE](LICENSE). Third-party dependencies retain their own licenses.
