# AGENTS.md

Guidance for coding agents working in this repository.

## Project

Rust SDK for building [Hasura Native Data Connectors](https://hasura.github.io/ndc-spec/).
Connector authors implement the `Connector` trait and get an executable that
serves the NDC HTTP API. This is a library consumed by downstream connectors,
so treat public APIs, re-exports, and enabled dependency features as part of
the contract.

Workspace crates:

- `crates/sdk-core` (`ndc_sdk_core`): the `Connector` trait, errors, state,
  schema, and JSON response helpers. Kept lightweight for CLI plugins.
- `crates/sdk` (`ndc_sdk`): the server built on axum — `default_main`, health
  checks, Prometheus metrics, OpenTelemetry tracing. Re-exports `ndc_models`
  as `models` and the core `connector`, `json_response`, and `state` modules.
- `crates/sdk/bin/main.rs`: the `ndc_hub_example` example connector.

`ndc-models` and `ndc-test` come from the `hasura/ndc-spec` git repo and must
be pinned to the same tag.

## Commands

The toolchain is pinned in `rust-toolchain.toml` (Rust 1.89.0). A Nix dev
shell is available (`nix develop`, or direnv via `.envrc`).

```sh
just check          # format-check, build, lint, test
just build          # cargo build --all-targets --all-features
just lint           # cargo clippy --all-targets --all-features && cargo machete --with-metadata
just test           # cargo test --all-targets --all-features
just format         # cargo fmt, nix fmt, prettier (when installed)
```

Run the example connector:

```sh
mkdir -p tmp/empty
cargo run --bin ndc_hub_example -- serve --configuration ./tmp/empty
```

## CI expectations

Changes must pass all of these (see `.github/workflows/`):

- `cargo fmt --all --check`, `nixpkgs-fmt --check .`, and `prettier --check .`.
- `cargo clippy --release --all-targets --all-features` with no warnings.
  Workspace lints enable `clippy::all` and `clippy::pedantic`; fix warnings
  rather than adding `allow`s unless there is a clear reason.
- `cargo test --release --all-targets --all-features`.
- `cargo machete --with-metadata` finds no unused dependencies. Dependencies
  kept only as version floors go in `[package.metadata.cargo-machete] ignored`
  with a comment explaining why.
- The build must not modify `Cargo.lock`. Commit lockfile changes together
  with the `Cargo.toml` changes that cause them.
- `openssl` must not become a dependency of `ndc-sdk-core` (checked with
  `cargo tree -i openssl --no-default-features -e no-dev`).
- `nix build` and `nix flake check` succeed.

## Dependencies

- Declare versions and features in the root `[workspace.dependencies]`, and
  use `{ workspace = true }` in crate manifests. Keep feature lists in one
  place rather than repeating them per crate.
- Downstream connectors do not use this repo's lockfile. For security fixes,
  raise the minimum version in `Cargo.toml`, not just `Cargo.lock`.
- Do not drop features of re-exported or commonly shared dependencies
  without checking downstream impact. For example, connectors rely on the
  `metrics` and `logs` features of `opentelemetry-otlp`.
- When upgrading, check the new version's feature list; renamed or removed
  features (for example `tls` → `tls-ring` in `opentelemetry-otlp` 0.33) fail
  dependency resolution.

## Tracing

- `crates/sdk/src/tracing.rs` uses `SdkTracerProvider::with_batch_exporter`,
  which exports from an SDK-owned thread without a Tokio runtime. OTLP/HTTP
  must use the blocking reqwest client (`opentelemetry-otlp`'s default
  `reqwest-blocking-client`). Do not enable `reqwest-client` or
  `hyper-client`.
- B3 propagation uses `opentelemetry-propagator-b3`; `opentelemetry-zipkin`
  is deprecated.
- `init_tracing` installs a global subscriber and reads `OTEL_*` environment
  variables. Each tracing test belongs in its own integration-test binary
  under `crates/sdk/tests/`. Tests should exercise real exports (emit a span,
  `force_flush`, assert a local collector received it), not just
  initialization.

## Changelog and releases

- Every user-facing change gets an entry in `CHANGELOG.md`, under the version
  section being prepared (or `[Unreleased]`). Describe impact on connector
  authors, especially breaking changes and dependency major-version bumps.
- Releases are cut by pushing a `v<version>` tag; the release workflow takes
  the GitHub release notes from the matching `CHANGELOG.md` section, so the
  heading must match `package.version` in the root `Cargo.toml`.
