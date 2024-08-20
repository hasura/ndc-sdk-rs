use std::borrow::ToOwned;
use std::env;
use std::error::Error;
use std::time::Duration;

use axum::body::Body;
use http::{Request, Response};
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{SpanExporterBuilder, WithExportConfig};
use tracing::{Level, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_tracing(
    service_name: Option<&str>,
    otlp_endpoint: Option<&str>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let trace_endpoint = otlp_endpoint
        .map(ToOwned::to_owned)
        .or_else(|| env::var(opentelemetry_otlp::OTEL_EXPORTER_OTLP_TRACES_ENDPOINT).ok());

    let log_level = env::var("RUST_LOG").unwrap_or(Level::INFO.to_string());
    let subscriber = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::builder()
                .parse(format!("{log_level},otel::tracing=trace,otel=debug"))?,
        )
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_timer(tracing_subscriber::fmt::time::time()),
        );

    match trace_endpoint {
        // disable traces exporter if the endpoint is empty
        None => subscriber.init(),
        Some(endpoint) => {
            opentelemetry::global::set_text_map_propagator(
                opentelemetry::propagation::composite::TextMapCompositePropagator::new(vec![
                    Box::new(opentelemetry_sdk::propagation::TraceContextPropagator::new()),
                    Box::new(opentelemetry_zipkin::Propagator::new()),
                ]),
            );

            let service_name = service_name.unwrap_or(env!("CARGO_PKG_NAME"));

            let exporter: SpanExporterBuilder =
                match env::var(opentelemetry_otlp::OTEL_EXPORTER_OTLP_PROTOCOL) {
                    Ok(protocol) => match protocol.as_str() {
                        "grpc" => Ok(opentelemetry_otlp::new_exporter()
                            .tonic()
                            .with_endpoint(endpoint)
                            .into()),
                        "http/protobuf" => Ok(opentelemetry_otlp::new_exporter()
                            .http()
                            .with_endpoint(endpoint)
                            .into()),
                        invalid => Err(format!("invalid protocol: {invalid:?}")),
                    },
                    // the default exporter protocol is grpc
                    Err(env::VarError::NotPresent) => Ok(opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(endpoint)
                        .into()),
                    Err(env::VarError::NotUnicode(os_str)) => {
                        Err(format!("invalid protocol: {os_str:?}"))
                    }
                }?;

            let provider = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(exporter)
                .with_trace_config(
                    opentelemetry_sdk::trace::Config::default()
                        .with_resource(opentelemetry_sdk::Resource::new(vec![
                            opentelemetry::KeyValue::new(
                                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                                service_name.to_string(),
                            ),
                            opentelemetry::KeyValue::new(
                                opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
                                env!("CARGO_PKG_VERSION"),
                            ),
                        ]))
                        .with_sampler(opentelemetry_sdk::trace::Sampler::ParentBased(Box::new(
                            opentelemetry_sdk::trace::Sampler::AlwaysOn,
                        ))),
                )
                .install_batch(opentelemetry_sdk::runtime::Tokio)?;

            let tracer = provider.tracer("ndc");

            subscriber
                .with(
                    tracing_opentelemetry::layer()
                        .with_error_records_to_exceptions(true)
                        .with_tracer(tracer),
                )
                .init();
        }
    };

    Ok(())
}
// Custom function for creating request-level spans
// tracing crate requires all fields to be defined at creation time, so any fields that will be set
// later should be defined as Empty
pub fn make_span(request: &Request<Body>) -> Span {
    use opentelemetry::trace::TraceContextExt;

    let span = tracing::info_span!(
        "request",
        method = %request.method(),
        uri = %request.uri(),
        version = ?request.version(),
        status = tracing::field::Empty,
        latency = tracing::field::Empty,
    );

    // Get parent trace id from headers, if available
    // This uses OTel extension set_parent rather than setting field directly on the span to ensure
    // it works no matter which propagator is configured
    let parent_context = opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.extract(&opentelemetry_http::HeaderExtractor(request.headers()))
    });
    // if there is no parent span ID, we get something nonsensical, so we need to validate it
    // (yes, this is hilarious)
    let parent_context_span = parent_context.span();
    let parent_context_span_context = parent_context_span.span_context();
    if parent_context_span_context.is_valid() {
        span.set_parent(parent_context);
    }

    span
}

// Custom function for adding information to request-level span that is only available at response time.
pub fn on_response(response: &Response<Body>, latency: Duration, span: &Span) {
    span.record("status", tracing::field::display(response.status()));
    span.record("latency", tracing::field::display(latency.as_nanos()));
}
