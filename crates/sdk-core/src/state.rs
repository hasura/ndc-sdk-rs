use std::path::Path;
use std::sync::Arc;

use prometheus::Registry;
use tokio::sync::OnceCell;

use crate::connector::error::*;
use crate::connector::{Connector, ConnectorSetup};

/// Everything we need to keep in memory.
pub struct ServerState<C: Connector> {
    configuration: C::Configuration,
    state: Arc<ConnectorState<C>>,
    metrics: prometheus::Registry,
}

/// The connector state, which may or may not be initialized.
struct ConnectorState<C: Connector> {
    cell: OnceCell<C::State>,
    init_state: Box<dyn ConnectorSetup<Connector = C>>,
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
        init_state: impl ConnectorSetup<Connector = C> + 'static,
        metrics: prometheus::Registry,
    ) -> Self {
        Self {
            configuration,
            state: Arc::new(ConnectorState {
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

/// Initialize the server state from the configuration file.
pub async fn init_server_state<Setup: ConnectorSetup>(
    setup: Setup,
    config_directory: &Path,
) -> Result<ServerState<Setup::Connector>> {
    let metrics = Registry::new();
    let configuration = setup.parse_configuration(config_directory).await?;
    Ok(ServerState::new(configuration, setup, metrics))
}
