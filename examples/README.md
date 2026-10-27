# Examples

Run these from the repository root:

- `cargo run -p frame-terminal --example profiles` lists shells without starting them.
- `cargo run -p frame-agent-connectors --example embedded` creates and disposes
  an embedded agent runtime without contacting a provider.
- `cargo run -p frame-agent-connectors --bin frame-agent-server -- --help`
  describes the optional JSONL process entry point.

Runnable example sources live in each package's `examples/` directory, so they
are also included in that package's source archive. Live provider use requires
explicit host configuration; see the agent package README. Desktop-framework
examples and bridges belong in Frame's host integration packages.
