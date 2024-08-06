use prometheus::{Registry, TextEncoder};

use crate::connector::error::{ErrorResponse, Result};
use crate::connector::Connector;

pub fn fetch_metrics<C: Connector>(
    configuration: &C::Configuration,
    state: &C::State,
    metrics: &Registry,
) -> Result<String> {
    let encoder = TextEncoder::new();

    C::fetch_metrics(configuration, state)?;

    let metric_families = &metrics.gather();

    encoder
        .encode_to_string(metric_families)
        .map_err(ErrorResponse::from_error)
}
