# Frame Server architecture

Frame Server provides host-independent execution packages. Packages are grouped
by capability and implementation language under `packages/`: `terminal-rust`
and `agents-rust` initially. Creating a backend starts no external work.

Rust applications embed libraries directly. The agent crate additionally offers
an explicit JSONL executable. A terminal server protocol is not implemented.
Optional Tauri integrations live inside each Rust feature crate under the
`tauri` Cargo feature. The engines remain host-independent by default.
Frame's npm Tauri package supplies matching TypeScript adapters, with no Rust
source. Applications explicitly register native plugins and retain credentials,
authorization and application-specific request policy. The agent transport
delegates these decisions through `AgentHost`; it does not bypass them.
Core has no dependency on Frame Server. Other hosts can embed the same engines
through their own integration; no additional host is implemented here.
The terminal API accepts a trusted host-assigned owner identifier; a renderer
must never choose this identity. Hosts authenticate requests, restrict content
access, close sessions on host disposal, and supply credentials and policies.
Terminal output callbacks return an error on disconnect, which terminates the
associated shell. Dropping the engine terminates its remaining sessions.

Keep existing public crate names and host-independent APIs. The optional
adapters are enabled explicitly in Cargo and registered on the Tauri builder.
Pixen uses the reusable agent transport while retaining artwork preparation,
tool restrictions, provider configuration, and application-exit cleanup.

The frontend and native integration must be released together before consumer
projects can switch from local development paths to published dependency pins.
Never publish changed crate contents under an existing immutable version.
Crates.io publication needs an authenticated account and available crate names.
GitHub Releases can carry verified .crate archives immediately; consumers can
use a pinned Git tag without a registry account. GitHub Packages is not Cargo.
