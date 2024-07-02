use crate::connector::{Connector, FetchMetricsError};
use prometheus::{Registry, TextEncoder};

pub fn fetch_metrics<C: Connector>(
    configuration: &C::Configuration,
    state: &C::State,
    metrics: &Registry,
) -> Result<String, FetchMetricsError> {
    let encoder = TextEncoder::new();

    C::fetch_metrics(configuration, state)?;

    let metric_families = &metrics.gather();

    encoder
        .encode_to_string(metric_families)
        .map_err(|_| FetchMetricsError::new("Unable to encode metrics"))
}
