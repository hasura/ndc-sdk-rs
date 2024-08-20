use crate::connector::Connector;

#[derive(Debug)]
pub struct ServerState<C: Connector> {
    configuration: C::Configuration,
    state: C::State,
    metrics: prometheus::Registry,
}

impl<C: Connector> Clone for ServerState<C>
where
    C::Configuration: Clone,
    C::State: Clone,
{
    fn clone(&self) -> Self {
        Self {
            configuration: self.configuration.clone(),
            state: self.state.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

impl<C: Connector> ServerState<C> {
    pub fn new(
        configuration: C::Configuration,
        state: C::State,
        metrics: prometheus::Registry,
    ) -> Self {
        Self {
            configuration,
            state,
            metrics,
        }
    }

    pub fn configuration(&self) -> &C::Configuration {
        &self.configuration
    }

    pub fn state(&self) -> &C::State {
        &self.state
    }

    pub fn metrics(&self) -> &prometheus::Registry {
        &self.metrics
    }
}
