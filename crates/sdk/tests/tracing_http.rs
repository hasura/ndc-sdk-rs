use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::routing::post;

// Keep this in its own integration-test binary: init_tracing installs a global
// subscriber, and the exporter protocol is configured through the environment.
//
// The batch span processor exports from its own thread without a Tokio runtime,
// so this test emits a span and checks that a local collector receives it.
#[tokio::test(flavor = "multi_thread")]
async fn exports_spans_over_http_protobuf() {
    std::env::set_var("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf");

    let requests = Arc::new(AtomicUsize::new(0));
    let collector = axum::Router::new().route(
        "/v1/traces",
        post({
            let requests = requests.clone();
            move || async move {
                requests.fetch_add(1, Ordering::SeqCst);
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind the collector");
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, collector).await });

    let provider = ndc_sdk::tracing::init_tracing(
        Some("http-tracing-regression-test"),
        Some(&format!("http://{address}/v1/traces")),
        "test-connector",
        "0.0.0",
    )
    .expect("HTTP/protobuf tracing should initialize without NoHttpClient")
    .expect("an explicit endpoint should enable the exporter");

    tracing::info_span!("http-tracing-regression-span").in_scope(|| {});

    // Flushing and shutting down block on the batch processor thread, so keep
    // them off the async workers that serve the collector.
    tokio::task::spawn_blocking(move || {
        provider.force_flush().expect("spans should be exported");
        provider
            .shutdown_with_timeout(Duration::from_secs(5))
            .expect("tracing should shut down cleanly");
    })
    .await
    .unwrap();

    assert!(
        requests.load(Ordering::SeqCst) > 0,
        "the collector should receive the exported span"
    );
}
