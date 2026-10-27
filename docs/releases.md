# Releases and package distribution

GitHub hosts this repository and versioned releases. GitHub Packages does not
provide a Cargo registry. Consumers can depend on a Git tag immediately after
it is pushed; they do not need a crates.io release or an npm alias.

For a release, update package versions and CHANGELOG.md together, run the checks
in CONTRIBUTING.md, commit, and push. Push a matching tag such as `v0.1.0`.
The release workflow verifies the tag version, tests the workspace on Windows
and Linux, builds and verifies source packages, and attaches the `.crate`
archives to a GitHub Release. A manual workflow run only checks and packages;
it does not publish a release. No compiled platform binaries are distributed yet.

Example consumer dependency:

```toml
frame-agent-connectors = { git = "https://github.com/poziel/frame-server.git", tag = "v0.1.0" }
```

Cargo.lock records the resolved Git commit. For local development, use a path
to the exact package directory instead. To inspect a downloaded source archive,
extract it and use that directory as a Cargo path dependency; `.crate` downloads
are source packages, not native binaries or a Cargo registry.

Publishing to crates.io is a separate explicit operation. Confirm crate-name
availability, package contents, metadata, license notices, and `cargo publish
--dry-run` first. Authenticate your crates.io account and publish each package
with `cargo publish -p PACKAGE`. Crates.io publication is permanent and package
versions cannot be overwritten. This project does not assume a publishing token
or claim registry availability before publication is verified.
