use crate::connector::Connector;

#[derive(Debug)]
pub struct ServerState<C: Connector> {
    pub configuration: C::Configuration,
    pub state: C::State,
    pub metrics: prometheus::Registry,
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
}
