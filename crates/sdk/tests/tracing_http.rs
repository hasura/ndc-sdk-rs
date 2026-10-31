// Keep this in its own integration-test binary: init_tracing installs a global
// subscriber, and the exporter protocol is configured through the environment.
#[tokio::test]
async fn initializes_http_protobuf_tracing() {
    std::env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");

    // Constructing the exporter does not require a running collector. No spans
    // are emitted, so this test does not send requests to the endpoint.
    let provider = ndc_sdk::tracing::init_tracing(
        Some("http-tracing-regression-test"),
        Some("http://127.0.0.1:4318/v1/traces"),
        "test-connector",
        "0.0.0",
    )
    .expect("HTTP/protobuf tracing should initialize without NoHttpClient")
    .expect("an explicit endpoint should enable the exporter");

    provider
        .shutdown()
        .expect("tracing should shut down cleanly");
}
