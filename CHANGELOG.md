# NDC Rust SDK Changelog

This changelog documents the changes between release versions.

## [Unreleased]

Changes to be included in the next upcoming release

## [0.6.0]

**Breaking changes** ([#38](https://github.com/hasura/ndc-sdk-rs/pull/38), [#40](https://github.com/hasura/ndc-sdk-rs/pull/40), [#41](https://github.com/hasura/ndc-sdk-rs/pull/41), [#42](https://github.com/hasura/ndc-sdk-rs/pull/42)):

- Updated to support [v0.2.0 of the NDC Spec](https://hasura.github.io/ndc-spec/specification/changelog.html#020). This is a very large update which adds new features and some breaking changes.
- If the [`X-Hasura-NDC-Version`](https://hasura.github.io/ndc-spec/specification/versioning.html) header is sent, the SDK will validate that the connector supports the incoming request's version and reject it if it does not. If no header is sent, no action is taken.

## [0.5.0] - 2024-10-29

- A default request size limit of 100MB was added. This can be overridden with the `HASURA_MAX_REQUEST_SIZE` environment variable ([#29](https://github.com/hasura/ndc-sdk-rs/pull/29)).
- Connector state is now only initialized on the first request that actually uses it. This means `/capabilities`, `/schema` and `/health` can be used even if state initialization would otherwise fail ([#31](https://github.com/hasura/ndc-sdk-rs/pull/31)).
- Add utilities to [implement PrintSchemaAndCapabilities](https://github.com/hasura/ndc-sdk-rs/pull/34). This splits the sdk into multiple crates to avoid bringing in openssl

## [0.4.0] - 2024-08-30

- update ndc-spec to v0.1.6 by @soupi in https://github.com/hasura/ndc-sdk-rs/pull/28

## [0.3.0] - 2024-08-12

- Health checks are now readiness checks; they should not make requests to any external services. We will revisit liveness and connectedness checks in a future release.
- The `/health` endpoint is now unsecured.

## [0.2.2] - 2024-07-30

- listen on all ipv4 and ipv6 interfaces by default (https://github.com/hasura/ndc-sdk-rs/pull/22)

## [0.2.1] - 2024-07-11

- Fix dynamic error types not being thread-safe

## [0.2.0] - 2024-07-09

- Update to `ndc-spec` v0.1.5
- Changed `get_capabilities` method in `Connector` trait so that `ndc-spec` version is obtained directly from the `ndc-spec` package instead of requiring the connector to specify it.
