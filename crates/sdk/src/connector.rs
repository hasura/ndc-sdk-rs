use std::error::Error;
use std::fmt::Display;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;
use ndc_models as models;
use serde::Serialize;
use thiserror::Error;

use crate::json_response::JsonResponse;

pub mod example;

/// Errors which occur when trying to validate connector
/// configuration.
///
/// See [`Connector::parse_configuration`].
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("error parsing configuration: {0}")]
    ParseError(LocatedError),
    #[error("error validating configuration: {0}")]
    ValidateError(InvalidNodes),
    #[error("could not find configuration file: {0}")]
    CouldNotFindConfiguration(PathBuf),
    #[error("error processing configuration: {0}")]
    IoError(#[from] std::io::Error),
    #[error("error processing configuration: {0}")]
    Other(#[from] Box<dyn Error + Send + Sync>),
}

/// An error associated with the position of a single character in a text file.
#[derive(Debug, Clone)]
pub struct LocatedError {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl Display for LocatedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{0}:{1}:{2}: {3}",
            self.file_path.display(),
            self.line,
            self.column,
            self.message
        )
    }
}

/// An error associated with a node in a graph structure.
#[derive(Debug, Clone)]
pub struct InvalidNode {
    pub file_path: PathBuf,
    pub node_path: Vec<KeyOrIndex>,
    pub message: String,
}

impl Display for InvalidNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, at ", self.file_path.display())?;
        for segment in &self.node_path {
            write!(f, ".{segment}")?;
        }
        write!(f, ": {}", self.message)?;
        Ok(())
    }
}

/// A set of invalid nodes.
#[derive(Debug, Clone)]
pub struct InvalidNodes(pub Vec<InvalidNode>);

impl Display for InvalidNodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iterator = self.0.iter();
        if let Some(first) = iterator.next() {
            first.fmt(f)?;
        }
        for next in iterator {
            write!(f, ", {next}")?;
        }
        Ok(())
    }
}

/// A segment in a node path, used with [InvalidNode].
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum KeyOrIndex {
    Key(String),
    Index(u32),
}

impl Display for KeyOrIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Key(key) => write!(f, "[{key:?}]"),
            Self::Index(index) => write!(f, "[{index}]"),
        }
    }
}

/// Errors which occur when trying to initialize connector
/// state.
///
/// See [`Connector::try_init_state`].
#[derive(Debug, Error)]
pub enum InitializationError {
    #[error("error initializing connector state: {0}")]
    Other(#[from] Box<dyn Error + Send + Sync>),
}

/// Errors which occur when trying to update metrics.
///
/// See [`Connector::fetch_metrics`].
#[derive(Debug, Error)]
pub enum FetchMetricsError {
    #[error("error fetching metrics: {0}")]
    Other(Box<dyn Error>, serde_json::Value),
}

impl FetchMetricsError {
    pub fn new<E: Into<Box<dyn Error>>>(err: E) -> Self {
        Self::Other(err.into(), serde_json::Value::Null)
    }
    #[must_use]
    pub fn with_details(self, details: serde_json::Value) -> Self {
        match self {
            Self::Other(err, _) => Self::Other(err, details),
        }
    }
}

impl IntoResponse for FetchMetricsError {
    fn into_response(self) -> Response {
        match self {
            Self::Other(err, details) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(models::ErrorResponse {
                    message: err.to_string(),
                    details,
                }),
            ),
        }
        .into_response()
    }
}

/// Errors which occur when checking connector health.
///
/// See [`Connector::health_check`].
#[derive(Debug, Error)]
pub enum HealthError {
    #[error("error checking health status: {0}")]
    Other(Box<dyn Error>, serde_json::Value),
}

impl HealthError {
    pub fn new<E: Into<Box<dyn Error>>>(err: E) -> Self {
        Self::Other(err.into(), serde_json::Value::Null)
    }
    #[must_use]
    pub fn with_details(self, details: serde_json::Value) -> Self {
        match self {
            Self::Other(err, _) => Self::Other(err, details),
        }
    }
}

impl IntoResponse for HealthError {
    fn into_response(self) -> Response {
        match self {
            Self::Other(err, details) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(models::ErrorResponse {
                    message: err.to_string(),
                    details,
                }),
            ),
        }
        .into_response()
    }
}

/// Errors which occur when retrieving the connector schema.
///
/// See [`Connector::get_schema`].
#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("error retrieving the schema: {0}")]
    Other(Box<dyn Error>, serde_json::Value),
}

impl SchemaError {
    pub fn new<E: Into<Box<dyn Error>>>(err: E) -> Self {
        Self::Other(err.into(), serde_json::Value::Null)
    }
    #[must_use]
    pub fn with_details(self, details: serde_json::Value) -> Self {
        match self {
            Self::Other(err, _) => Self::Other(err, details),
        }
    }
}

impl From<Box<dyn Error>> for SchemaError {
    fn from(value: Box<dyn Error>) -> Self {
        Self::new(value)
    }
}

impl IntoResponse for SchemaError {
    fn into_response(self) -> Response {
        match self {
            Self::Other(err, details) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(models::ErrorResponse {
                    message: err.to_string(),
                    details,
                }),
            ),
        }
        .into_response()
    }
}

/// Errors which occur when executing a query.
///
/// See [`Connector::query`].
#[derive(Debug, Error)]
pub enum QueryError {
    /// The request was invalid or did not match the
    /// requirements of the specification. This indicates
    /// an error with the client.
    #[error("invalid request: {}", .0.message)]
    InvalidRequest(models::ErrorResponse),
    /// The request was well formed but was unable to be
    /// followed due to semantic errors. This indicates
    /// an error with the client.
    #[error("unprocessable content: {}", .0.message)]
    UnprocessableContent(models::ErrorResponse),
    /// The request relies on an unsupported feature or
    /// capability. This may indicate an error with the client,
    /// or just an unimplemented feature.
    #[error("unsupported operation: {}", .0.message)]
    UnsupportedOperation(models::ErrorResponse),
    #[error("error executing query: {0}")]
    Other(Box<dyn Error>, serde_json::Value),
}

impl QueryError {
    pub fn new<E: Into<Box<dyn Error>>>(err: E) -> Self {
        Self::Other(err.into(), serde_json::Value::Null)
    }
    pub fn new_invalid_request<T: ToString>(message: &T) -> Self {
        Self::InvalidRequest(models::ErrorResponse {
            message: message.to_string(),
            details: serde_json::Value::Null,
        })
    }
    pub fn new_unprocessable_content<T: ToString>(message: &T) -> Self {
        Self::UnprocessableContent(models::ErrorResponse {
            message: message.to_string(),
            details: serde_json::Value::Null,
        })
    }
    pub fn new_unsupported_operation<T: ToString>(message: &T) -> Self {
        Self::UnsupportedOperation(models::ErrorResponse {
            message: message.to_string(),
            details: serde_json::Value::Null,
        })
    }
    #[must_use]
    pub fn with_details(self, details: serde_json::Value) -> Self {
        match self {
            Self::InvalidRequest(models::ErrorResponse { message, .. }) => {
                Self::InvalidRequest(models::ErrorResponse { message, details })
            }
            Self::UnprocessableContent(models::ErrorResponse { message, .. }) => {
                Self::UnprocessableContent(models::ErrorResponse { message, details })
            }
            Self::UnsupportedOperation(models::ErrorResponse { message, .. }) => {
                Self::UnsupportedOperation(models::ErrorResponse { message, details })
            }
            Self::Other(err, _) => Self::Other(err, details),
        }
    }
}

impl From<Box<dyn Error>> for QueryError {
    fn from(value: Box<dyn Error>) -> Self {
        Self::new(value)
    }
}

impl IntoResponse for QueryError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidRequest(err) => (StatusCode::BAD_REQUEST, Json(err)),
            Self::UnprocessableContent(err) => (StatusCode::UNPROCESSABLE_ENTITY, Json(err)),
            Self::UnsupportedOperation(err) => (StatusCode::NOT_IMPLEMENTED, Json(err)),
            Self::Other(err, details) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(models::ErrorResponse {
                    message: err.to_string(),
                    details,
                }),
            ),
        }
        .into_response()
    }
}

/// Errors which occur when explaining a query.
///
/// See [`Connector::query_explain`, `Connector::mutation_explain`].
#[derive(Debug, Error)]
pub enum ExplainError {
    /// The request was invalid or did not match the
    /// requirements of the specification. This indicates
    /// an error with the client.
    #[error("invalid request: {}", .0.message)]
    InvalidRequest(models::ErrorResponse),
    /// The request was well formed but was unable to be
    /// followed due to semantic errors. This indicates
    /// an error with the client.
    #[error("unprocessable content: {}", .0.message)]
    UnprocessableContent(models::ErrorResponse),
    /// The request relies on an unsupported feature or
    /// capability. This may indicate an error with the client,
    /// or just an unimplemented feature.
    #[error("unsupported operation: {}", .0.message)]
    UnsupportedOperation(models::ErrorResponse),
    #[error("explain error: {0}")]
    Other(Box<dyn Error>, serde_json::Value),
}

impl ExplainError {
    pub fn new<E: Into<Box<dyn Error>>>(err: E) -> Self {
        Self::Other(err.into(), serde_json::Value::Null)
    }
    pub fn new_invalid_request<T: ToString>(message: &T) -> Self {
        Self::InvalidRequest(models::ErrorResponse {
            message: message.to_string(),
            details: serde_json::Value::Null,
        })
    }
    pub fn new_unprocessable_content<T: ToString>(message: &T) -> Self {
        Self::UnprocessableContent(models::ErrorResponse {
            message: message.to_string(),
            details: serde_json::Value::Null,
        })
    }
    pub fn new_unsupported_operation<T: ToString>(message: &T) -> Self {
        Self::UnsupportedOperation(models::ErrorResponse {
            message: message.to_string(),
            details: serde_json::Value::Null,
        })
    }
    #[must_use]
    pub fn with_details(self, details: serde_json::Value) -> Self {
        match self {
            Self::InvalidRequest(models::ErrorResponse { message, .. }) => {
                Self::InvalidRequest(models::ErrorResponse { message, details })
            }
            Self::UnprocessableContent(models::ErrorResponse { message, .. }) => {
                Self::UnprocessableContent(models::ErrorResponse { message, details })
            }
            Self::UnsupportedOperation(models::ErrorResponse { message, .. }) => {
                Self::UnsupportedOperation(models::ErrorResponse { message, details })
            }
            Self::Other(err, _) => Self::Other(err, details),
        }
    }
}

impl From<Box<dyn Error>> for ExplainError {
    fn from(value: Box<dyn Error>) -> Self {
        Self::new(value)
    }
}

impl IntoResponse for ExplainError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidRequest(err) => (StatusCode::BAD_REQUEST, Json(err)),
            Self::UnprocessableContent(err) => (StatusCode::UNPROCESSABLE_ENTITY, Json(err)),
            Self::UnsupportedOperation(err) => (StatusCode::NOT_IMPLEMENTED, Json(err)),
            Self::Other(err, details) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(models::ErrorResponse {
                    message: err.to_string(),
                    details,
                }),
            ),
        }
        .into_response()
    }
}

/// Errors which occur when executing a mutation.
///
/// See [`Connector::mutation`].
#[derive(Debug, Error)]
pub enum MutationError {
    /// The request was invalid or did not match the
    /// requirements of the specification. This indicates
    /// an error with the client.
    #[error("invalid request: {}", .0.message)]
    InvalidRequest(models::ErrorResponse),
    /// The request was well formed but was unable to be
    /// followed due to semantic errors. This indicates
    /// an error with the client.
    #[error("unprocessable content: {}", .0.message)]
    UnprocessableContent(models::ErrorResponse),
    /// The request relies on an unsupported feature or
    /// capability. This may indicate an error with the client,
    /// or just an unimplemented feature.
    #[error("unsupported operation: {}", .0.message)]
    UnsupportedOperation(models::ErrorResponse),
    /// The request would result in a conflicting state
    /// in the underlying data store.
    #[error("mutation would result in a conflicting state: {}", .0.message)]
    Conflict(models::ErrorResponse),
    /// The request would violate a constraint in the
    /// underlying data store.
    #[error("mutation violates constraint: {}", .0.message)]
    ConstraintNotMet(models::ErrorResponse),
    #[error("error executing mutation: {0}")]
    Other(Box<dyn Error>, serde_json::Value),
}

impl MutationError {
    pub fn new<E: Into<Box<dyn Error>>>(err: E) -> Self {
        Self::Other(err.into(), serde_json::Value::Null)
    }
    pub fn new_invalid_request<T: ToString>(message: &T) -> Self {
        Self::InvalidRequest(models::ErrorResponse {
            message: message.to_string(),
            details: serde_json::Value::Null,
        })
    }
    pub fn new_unprocessable_content<T: ToString>(message: &T) -> Self {
        Self::UnprocessableContent(models::ErrorResponse {
            message: message.to_string(),
            details: serde_json::Value::Null,
        })
    }
    pub fn new_unsupported_operation<T: ToString>(message: &T) -> Self {
        Self::UnsupportedOperation(models::ErrorResponse {
            message: message.to_string(),
            details: serde_json::Value::Null,
        })
    }
    pub fn new_conflict<T: ToString>(message: &T) -> Self {
        Self::Conflict(models::ErrorResponse {
            message: message.to_string(),
            details: serde_json::Value::Null,
        })
    }
    pub fn new_constraint_not_met<T: ToString>(message: &T) -> Self {
        Self::ConstraintNotMet(models::ErrorResponse {
            message: message.to_string(),
            details: serde_json::Value::Null,
        })
    }
    #[must_use]
    pub fn with_details(self, details: serde_json::Value) -> Self {
        match self {
            Self::InvalidRequest(models::ErrorResponse { message, .. }) => {
                Self::InvalidRequest(models::ErrorResponse { message, details })
            }
            Self::UnprocessableContent(models::ErrorResponse { message, .. }) => {
                Self::UnprocessableContent(models::ErrorResponse { message, details })
            }
            Self::UnsupportedOperation(models::ErrorResponse { message, .. }) => {
                Self::UnsupportedOperation(models::ErrorResponse { message, details })
            }
            Self::Conflict(models::ErrorResponse { message, .. }) => {
                Self::Conflict(models::ErrorResponse { message, details })
            }
            Self::ConstraintNotMet(models::ErrorResponse { message, .. }) => {
                Self::ConstraintNotMet(models::ErrorResponse { message, details })
            }
            Self::Other(err, _) => Self::Other(err, details),
        }
    }
}

impl From<Box<dyn Error>> for MutationError {
    fn from(value: Box<dyn Error>) -> Self {
        Self::new(value)
    }
}

impl IntoResponse for MutationError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidRequest(err) => (StatusCode::BAD_REQUEST, Json(err)),
            Self::UnprocessableContent(err) => (StatusCode::UNPROCESSABLE_ENTITY, Json(err)),
            Self::UnsupportedOperation(err) => (StatusCode::NOT_IMPLEMENTED, Json(err)),
            Self::Conflict(err) => (StatusCode::CONFLICT, Json(err)),
            Self::ConstraintNotMet(err) => (StatusCode::FORBIDDEN, Json(err)),
            Self::Other(err, details) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(models::ErrorResponse {
                    message: err.to_string(),
                    details,
                }),
            ),
        }
        .into_response()
    }
}

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
pub trait Connector {
    /// The type of validated configuration
    type Configuration: Sync + Send;
    /// The type of unserializable state
    type State: Sync + Send;

    /// Update any metrics from the state
    ///
    /// Note: some metrics can be updated directly, and do not
    /// need to be updated here. This function can be useful to
    /// query metrics which cannot be updated directly, e.g.
    /// the number of idle connections in a connection pool
    /// can be polled but not updated directly.
    fn fetch_metrics(
        configuration: &Self::Configuration,
        state: &Self::State,
    ) -> Result<(), FetchMetricsError>;

    /// Check the health of the connector.
    ///
    /// For example, this function should check that the connector
    /// is able to reach its data source over the network.
    async fn health_check(
        configuration: &Self::Configuration,
        state: &Self::State,
    ) -> Result<(), HealthError>;

    /// Get the connector's capabilities.
    ///
    /// This function implements the [capabilities endpoint](https://hasura.github.io/ndc-spec/specification/capabilities.html)
    /// from the NDC specification.
    async fn get_capabilities() -> JsonResponse<models::CapabilitiesResponse>;

    /// Get the connector's schema.
    ///
    /// This function implements the [schema endpoint](https://hasura.github.io/ndc-spec/specification/schema/index.html)
    /// from the NDC specification.
    async fn get_schema(
        configuration: &Self::Configuration,
    ) -> Result<JsonResponse<models::SchemaResponse>, SchemaError>;

    /// Explain a query by creating an execution plan
    ///
    /// This function implements the [query/explain endpoint](https://hasura.github.io/ndc-spec/specification/explain.html)
    /// from the NDC specification.
    async fn query_explain(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>, ExplainError>;

    /// Explain a mutation by creating an execution plan
    ///
    /// This function implements the [mutation/explain endpoint](https://hasura.github.io/ndc-spec/specification/explain.html)
    /// from the NDC specification.
    async fn mutation_explain(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::MutationRequest,
    ) -> Result<JsonResponse<models::ExplainResponse>, ExplainError>;

    /// Execute a mutation
    ///
    /// This function implements the [mutation endpoint](https://hasura.github.io/ndc-spec/specification/mutations/index.html)
    /// from the NDC specification.
    async fn mutation(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::MutationRequest,
    ) -> Result<JsonResponse<models::MutationResponse>, MutationError>;

    /// Execute a query
    ///
    /// This function implements the [query endpoint](https://hasura.github.io/ndc-spec/specification/queries/index.html)
    /// from the NDC specification.
    async fn query(
        configuration: &Self::Configuration,
        state: &Self::State,
        request: models::QueryRequest,
    ) -> Result<JsonResponse<models::QueryResponse>, QueryError>;
}

/// Connectors are set up by values that implement this trait.
///
/// It provides a method for parsing configuration, and another for initializing
/// state.
///
/// See [`Connector`] for further details.
#[async_trait]
pub trait ConnectorSetup {
    type Connector: Connector;

    /// Validate the configuration provided by the user, returning a
    /// configuration error or a validated [`Connector::Configuration`].
    async fn parse_configuration(
        &self,
        configuration_dir: impl AsRef<Path> + Send,
    ) -> Result<<Self::Connector as Connector>::Configuration, ParseError>;

    /// Initialize the connector's in-memory state.
    ///
    /// For example, any connection pools, prepared queries, or other managed
    /// resources would be allocated here.
    ///
    /// In addition, this function should register any connector-specific
    /// metrics with the metrics registry.
    async fn try_init_state(
        &self,
        configuration: &<Self::Connector as Connector>::Configuration,
        metrics: &mut prometheus::Registry,
    ) -> Result<<Self::Connector as Connector>::State, InitializationError>;
}
