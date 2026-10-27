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

Both crates were initially published at `0.1.0` through a verified local Cargo
upload. Confirm package contents, metadata, license notices, and
`cargo publish --dry-run` before each new release. Registry versions cannot be
overwritten; subsequent stable releases must increment the package version.

## crates.io automation

Trusted publishers are configured for both crates and
`CRATES_IO_PUBLISH_ENABLED=true` is enabled in the repository. The bootstrap
token used for the initial upload was revoked. No monthly token renewal or
stored Cargo publishing secret is required.

The publish job in `.github/workflows/rust.yml` uses crates.io trusted
publishing through `rust-lang/crates-io-auth-action`. No persistent publishing
secret belongs in GitHub Actions. Both Windows and Linux verification jobs must
succeed before uploading either crate.

After bootstrap setup, enable repository Actions variable
`CRATES_IO_PUBLISH_ENABLED` with value `true`. Pushes to `main` then publish both
crates with versions such as `0.1.0-dev.12.1`, based on the workflow run number
and attempt. Only CI manifests are changed; committed versions stay at `0.1.0`.
Pushes of matching version tags publish the stable workspace version. Manual
runs and pull requests only validate and package. Cargo consumers must select
prerelease versions explicitly; crates.io has no npm-style `next` tag.

### First publication

1. Sign in to crates.io and verify your email address.
2. Create a short-lived token permitting creation and publication of
   `frame-terminal` and `frame-agent-connectors`. Enter it through `cargo login`
   in your own terminal; never commit it or paste it into chat.
3. From a clean, verified checkout, run `cargo publish --workspace --locked`.
4. In each crate's crates.io settings, configure GitHub trusted publishing for
   owner `poziel`, repository `frame`, and workflow filename `rust.yml`, with no
   environment restriction (the job does not declare an environment).
5. Revoke the bootstrap token and run `cargo logout` if no other publishing
   operation needs the local credential.
6. Set `CRATES_IO_PUBLISH_ENABLED=true` in GitHub repository Actions variables,
   then push a commit to `main` to verify token-free development publication.

The maintained Server crates are `frame-terminal` and `frame-agent-connectors`.
The `tauri-plugin-frame-terminal` bridge belongs to the Frame repository and
needs its own release setup; it is not a member of this workspace.

Once publication is confirmed, registry dependencies use ordinary versions:

```toml
frame-terminal = "0.1.0"
frame-agent-connectors = "0.1.0"
```

Until then, the pinned Git dependencies above remain valid. See
https://crates.io/docs/trusted-publishing for the current registry setup rules.
