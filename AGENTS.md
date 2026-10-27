# Working on Frame Server

Read README.md and docs/architecture.md before changes. Preserve host-independent
Rust APIs and caller ownership of credentials, processes, configuration and
authorization. Hosts supply trusted identities; renderer input is not identity.

Use cargo fmt, cargo test --workspace, cargo package --workspace and the Python
helper tests. Document real verification limits. Keep network/provider tests
offline by default. Preserve unrelated work and recovery sources. Do not commit,
push, tag, or publish unless requested by the user.
