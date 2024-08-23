use crate::json_response::JsonResponse;
use async_trait::async_trait;
use ndc_models as models;
use std::path::Path;
pub mod error;
pub mod example;
pub use error::*;

/// Connectors using this library should implement this trait.
///
/// It provides methods which implement the standard endpoints defined by the
/// specification: capabilities, schema, query, mutation, query/explain,
/// and mutation/explain.
///
/// In addition, it introduces names for types to manage state and configuration
/// (if any), and provides any necessary context for observability purposes
/// (metrics, logging and tracing).
///
/// ## Configuration
///
/// Connectors encapsulate data sources, and likely require configuration
/// (connection strings, web service tokens, etc.). The NDC specification does
/// not discuss this sort of configuration, because it is an implementation
/// detail of a specific connector, but it is useful to adopt a convention here
/// for simplified configuration management.
///
/// Configuration is specified by the connector implementation. It is provided
/// as a path to a directory. Parsing this directory should result in a
/// [`Connector::Configuration`].
///
/// ## State
///
/// In addition to configuration, this trait defines a [`Connector::State`] type
/// for state management.
///
/// State is distinguished from configuration in that it is not provided
/// directly by the user, and would not ordinarily be serializable. For example,
/// a connection string would be configuration, but a connection pool object
/// created from that connection string would be state.
#[async_trait]
pub trait Connector: Send {
    /// The type of validated configuration
    type Configuration: Send + Sync;
    /// The type of unserializable state
    type State: Send + Sync;

    /// Update any metrics from the state
    ///
    /// Note: some metrics can be updated directly, and do not
    /// need to be updated here. This function can be useful to
    /// query metrics which cannot be updated directly, e.g.
    /// the number of idle connections in a connection pool
    /// can be polled but not updated directly.
    fn fetch_metrics(configuration: &Self::Configuration, state: &Self::State) -> Result<()>;

    /// Check the health of the connector.
    ///
    /// This should simply verify that the connector is ready to start accepting
    /// requests. It should not verify that external data sources are available.
    ///
    /// For most use cases, the default implementation should be fine.
    async fn get_health_readiness(
        _configuration: &Self::Configuration,
        _state: &Self::State,
    ) -> Result<()> {
        Ok(())
    }

    /// Get the connector's capabilities.
    ///
    /// This function implements the [capabilities endpoint](https://hasura.github.io/ndc-spec/specification/capabilities.html)
    /// from the NDC specification.
    async fn get_capabilities() -> models::Capabilities;

    /// Get the connector's schema.
    ///
    /// This function implements the [schema endpoint](https://hasura.github.io/ndc-spec/specification/schema/index.html)
    /// from the NDC specification.
    async fn get_schema(
        configuration: &Self::Configuration,
    ) -> Result<JsonResponse<models::SchemaResponse>>;

    /// Explain a query by creating an execution plan
    ///
    /// This function implements the [query/explain endpoint](https://hasura.github.io/ndc-spec/specification/explain.html)
    /// from the NDC specification.
    ///
    /// The [`QueryError`] type is provided as a convenience to connector authors, to be used on
    /// error.
    async fn query_explain(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>>;

    /// Explain a mutation by creating an execution plan
    ///
    /// This function implements the [mutation/explain endpoint](https://hasura.github.io/ndc-spec/specification/explain.html)
    /// from the NDC specification.
    ///
    /// The [`MutationError`] type is provided as a convenience to connector authors, to be used on
    /// error.
    async fn mutation_explain(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::MutationRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>>;

    /// Execute a mutation
    ///
    /// This function implements the [mutation endpoint](https://hasura.github.io/ndc-spec/specification/mutations/index.html)
    /// from the NDC specification.
    ///
    /// The [`MutationError`] type is provided as a convenience to connector authors, to be used on
    /// error.
    async fn mutation(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::MutationRequest,
    ) -> Result<JsonResponse<models::MutationResponse>>;

    /// Execute a query
    ///
    /// This function implements the [query endpoint](https://hasura.github.io/ndc-spec/specification/queries/index.html)
    /// from the NDC specification.
    ///
    /// The [`QueryError`] type is provided as a convenience to connector authors, to be used on
    /// error.
    async fn query(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<JsonResponse<models::QueryResponse>>;
}

/// Connectors are set up by values that implement this trait.
///
/// It provides a method for parsing configuration, and another for initializing state.
///
/// See [`Connector`] for further details.
//
// This is actually split into [`ParseConfiguration`] and [`InitState`], because this makes it
// possible to pass around a `Box<dyn InitState>` internally. This is not ideal and we would prefer
// to merge these back into a single trait.
#[async_trait]
pub trait ConnectorSetup:
    ParseConfiguration<Configuration = <Self::Connector as Connector>::Configuration>
    + InitState<
        Configuration = <Self::Connector as Connector>::Configuration,
        State = <Self::Connector as Connector>::State,
    > + 'static
{
    type Connector: Connector;
}

/// Reads configuration from a directory and returns the specified configuration.
#[async_trait]
pub trait ParseConfiguration {
    type Configuration;

    /// Validate the configuration provided by the user, returning a configuration error or a
    /// validated [`Configuration`].
    ///
    /// The [`ParseError`] type is provided as a convenience to connector authors, to be used on
    /// error.
    async fn parse_configuration(
        &self,
        configuration_dir: impl AsRef<Path> + Send,
    ) -> Result<Self::Configuration>;
}

/// Initializes the connector state.
#[async_trait]
pub trait InitState: Send + Sync {
    type Configuration;
    type State;

    /// Initialize the connector's in-memory state.
    ///
    /// For example, any connection pools, prepared queries, or other managed resources would be
    /// allocated here.
    ///
    /// In addition, this function should register any connector-specific metrics with the metrics
    /// registry.
    ///
    /// This may be called repeatedly until it succeeds.
    async fn try_init_state(
        &self,
        configuration: &Self::Configuration,
        metrics: &mut prometheus::Registry,
    ) -> Result<Self::State>;
}
