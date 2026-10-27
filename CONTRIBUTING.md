# Contributing

Keep execution packages independent of UI frameworks. Use capability-language
directory names under `packages/`. Add new languages only when an implementation
exists. Preserve public crate names unless a release explicitly migrates them.

Run `cargo fmt --all --check`, `cargo test --workspace`, `cargo package --workspace`,
and the Python helper tests described in README before proposing a change.
Keep tests offline with fake providers. Do not start authenticated provider
sessions or external tools merely by importing or constructing a backend.

Document public APIs, owner identity rules, resources, and disposal. Maintain
tests for owner isolation, disconnects, startup failure, and process cleanup.
Do not commit target directories, credentials, virtual environments, local
configuration, recovery trees, or generated verification output.
