# NDC TypeScript SDK Changelog

This changelog documents the changes between release versions.

## [Unreleased]

Changes to be included in the next upcoming release

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
