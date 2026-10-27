# Frame Server architecture

Frame Server provides host-independent execution packages. Packages are grouped
by capability and implementation language under `packages/`: `terminal-rust`
and `agents-rust` initially. Creating a backend starts no external work.

Rust applications embed libraries directly. The agent crate additionally offers
an explicit JSONL executable. A terminal server protocol is not implemented.
Host-specific bridges belong to Frame's host integration packages, including
the Rust and TypeScript halves of a Tauri bridge. Core has no dependency on
Frame Server. Other hosts can embed the same engine through their own bridge.

The terminal API accepts a trusted host-assigned owner identifier; a renderer
must never choose this identity. Hosts authenticate requests, restrict content
access, close sessions on host disposal, and supply credentials and policies.
Terminal output callbacks return an error on disconnect, which terminates the
associated shell. Dropping the engine terminates its remaining sessions.

Keep existing agent crate names and version 0.1.0. Copy active sources and tests
from the former companion; exclude archives, caches, and recovery trees. Retain
the original companion until consumers have migrated and verification passes.

## Implementation plan

1. Preserve the agent crate, Python helper, fixtures, and tests in agents-rust.
2. Test the terminal's public lifecycle and owner isolation, then extract PTY
   handling into terminal-rust and retain Windows startup/termination behavior.
3. Delegate Frame's Tauri plugin to this engine and expose its TypeScript
   transport through @frame/tauri/terminal. Preserve permissions and cleanup.
4. Add Cargo workspace, package metadata/licenses, examples, contribution and
   release documentation, CI, and versioned source package release artifacts.
5. Update Pixen's local agent dependency; verify Rust tests, package contents,
   bridge compilation, and TypeScript host builds before committing/pushing.

Crates.io publication needs an authenticated account and available crate names.
GitHub Releases can carry verified .crate archives immediately; consumers can
use a pinned Git tag without a registry account. GitHub Packages is not Cargo.
