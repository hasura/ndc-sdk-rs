use std::sync::Arc;

use tokio::sync::OnceCell;

use crate::connector::error::*;
use crate::connector::{Connector, InitState};

/// Everything we need to keep in memory.
pub struct ServerState<C: Connector> {
    configuration: C::Configuration,
    state: Arc<ApplicationState<C>>,
    metrics: prometheus::Registry,
}

/// The application state, which may or may not be initialized.
struct ApplicationState<C: Connector> {
    cell: OnceCell<C::State>,
    init_state: Box<dyn InitState<Configuration = C::Configuration, State = C::State>>,
}

// Server state must be cloneable even if the underlying connector is not.
// We only require `Connector::Configuration` to be cloneable.
//
// Server state is always stored in an `Arc`, so is therefore cloneable.
impl<C: Connector> Clone for ServerState<C>
where
    C::Configuration: Clone,
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
    /// Construct a new server state.
    pub fn new(
        configuration: C::Configuration,
        init_state: impl InitState<Configuration = C::Configuration, State = C::State> + 'static,
        metrics: prometheus::Registry,
    ) -> Self {
        Self {
            configuration,
            state: Arc::new(ApplicationState {
                cell: OnceCell::new(),
                init_state: Box::new(init_state),
            }),
            metrics,
        }
    }

    /// The server configuration.
    pub fn configuration(&self) -> &C::Configuration {
        &self.configuration
    }

    /// The transient server state.
    ///
    /// If the state has not yet been initialized, this initializes it.
    ///
    /// On initialization failure, this function will also fail, and subsequent calls will retry.
    pub async fn state(&self) -> Result<&C::State> {
        self.state
            .cell
            .get_or_try_init(|| async {
                self.state
                    .init_state
                    .try_init_state(&self.configuration, &mut self.metrics.clone())
                    .await
            })
            .await
    }

    /// The server metrics.
    pub fn metrics(&self) -> &prometheus::Registry {
        &self.metrics
    }
}
