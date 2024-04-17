# Hasura NDC SDK for Rust

This repository provides a Rust crate to aid development of [Hasura Native Data
Connectors](https://hasura.github.io/ndc-spec/). Developers can implement a
trait, and derive an executable which can be used to run a connector which is
compatible with the specification.

In addition, this library adopts certain conventions which are not covered by
the current specification:

- Connector configuration
- State management
- Trace collection

#### Getting Started with the SDK

```sh
cargo build
```

#### Run the example connector

```sh
mkdir empty
cargo run --bin ndc_hub_example -- --configuration ./empty
```

Inspect the resulting (empty) schema:

```sh
curl http://localhost:8080/schema
```

(The default port, 8080, can be changed using `--port`.)

## Tracing

The serve command emits OTLP trace information. This can be used to see details
of requests across services.

To enable tracing you must:

- use the SDK option `--otlp-endpoint` e.g. `http://localhost:4317`,
- set the SDK environment variable `OTEL_EXPORTER_OTLP_ENDPOINT`, or
- set the `tracing` environment variable `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`.

For additional service information you can:

- Set `OTEL_SERVICE_NAME` e.g. `ndc_hub_example`
- Set `OTEL_RESOURCE_ATTRIBUTES` e.g. `key=value, k = v, a= x, a=z`

To view trace information during local development you can run a Jaeger server via Docker:

```
docker run --name jaeger -e COLLECTOR_OTLP_ENABLED=true -p 16686:16686 -p 4317:4317 -p 4318:4318 jaegertracing/all-in-one
```
