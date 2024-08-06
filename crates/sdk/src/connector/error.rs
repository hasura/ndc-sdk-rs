use std::fmt::Display;
use std::path::PathBuf;

use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;
use serde::Serialize;

use ndc_models as models;

pub type Result<T> = std::result::Result<T, ErrorResponse>;

#[derive(Debug, Clone, thiserror::Error)]
pub struct ErrorResponse {
    status_code: StatusCode,
    inner: ndc_models::ErrorResponse,
}

impl ErrorResponse {
    pub fn new(status_code: StatusCode, message: String, details: serde_json::Value) -> Self {
        Self {
            status_code,
            inner: ndc_models::ErrorResponse { message, details },
        }
    }

    pub fn new_internal_with_details(details: serde_json::Value) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal error".to_string(),
            details,
        )
    }

    pub fn from_error<E: std::error::Error + Send + Sync + 'static>(value: E) -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            inner: ndc_models::ErrorResponse {
                message: value.to_string(),
                details: serde_json::Value::Null,
            },
        }
    }

    #[must_use]
    pub fn with_status_code(self, status_code: StatusCode) -> Self {
        Self {
            status_code,
            ..self
        }
    }
}

impl std::fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}\n(details: {})",
            self.status_code, self.inner.message, self.inner.details
        )
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ErrorResponse {
    fn from(value: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            inner: ndc_models::ErrorResponse {
                message: value.to_string(),
                details: serde_json::Value::Null,
            },
        }
    }
}

impl From<ndc_models::ErrorResponse> for ErrorResponse {
    fn from(value: ndc_models::ErrorResponse) -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            inner: value,
        }
    }
}

#[cfg(feature = "ndc-test")]
impl From<ErrorResponse> for ndc_test::error::Error {
    fn from(value: ErrorResponse) -> Self {
        Self::OtherError(Box::new(value))
    }
}

impl From<String> for ErrorResponse {
    fn from(value: String) -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            inner: ndc_models::ErrorResponse {
                message: value,
                details: serde_json::Value::Null,
            },
        }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        (self.status_code, Json(self.inner)).into_response()
    }
}

/// Errors which occur when trying to validate connector
/// configuration.
///
/// See [`Connector::parse_configuration`].
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("error parsing configuration: {0}")]
    ParseError(LocatedError),
    #[error("error validating configuration: {0}")]
    ValidateError(InvalidNodes),
    #[error("could not find configuration file: {0}")]
    CouldNotFindConfiguration(PathBuf),
    #[error("error processing configuration: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<ParseError> for ErrorResponse {
    fn from(value: ParseError) -> Self {
        Self::from_error(value)
    }
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

/// Errors which occur when executing a query.
///
/// See [`Connector::query`].
#[derive(Debug, thiserror::Error)]
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
}

impl QueryError {
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
        }
    }
}

impl From<QueryError> for ErrorResponse {
    fn from(value: QueryError) -> Self {
        match value {
            QueryError::InvalidRequest(err) => {
                ErrorResponse::from(err).with_status_code(StatusCode::BAD_REQUEST)
            }
            QueryError::UnprocessableContent(err) => {
                ErrorResponse::from(err).with_status_code(StatusCode::UNPROCESSABLE_ENTITY)
            }
            QueryError::UnsupportedOperation(err) => {
                ErrorResponse::from(err).with_status_code(StatusCode::NOT_IMPLEMENTED)
            }
        }
    }
}

/// Errors which occur when explaining a query.
///
/// See [`Connector::query_explain`, `Connector::mutation_explain`].
#[derive(Debug, thiserror::Error)]
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
}

impl ExplainError {
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
        }
    }
}

impl From<ExplainError> for ErrorResponse {
    fn from(value: ExplainError) -> Self {
        match value {
            ExplainError::InvalidRequest(err) => {
                ErrorResponse::from(err).with_status_code(StatusCode::BAD_REQUEST)
            }
            ExplainError::UnprocessableContent(err) => {
                ErrorResponse::from(err).with_status_code(StatusCode::UNPROCESSABLE_ENTITY)
            }
            ExplainError::UnsupportedOperation(err) => {
                ErrorResponse::from(err).with_status_code(StatusCode::NOT_IMPLEMENTED)
            }
        }
    }
}

/// Errors which occur when executing a mutation.
///
/// See [`Connector::mutation`].
#[derive(Debug, thiserror::Error)]
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
}

impl MutationError {
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
        }
    }
}

impl From<MutationError> for ErrorResponse {
    fn from(value: MutationError) -> Self {
        match value {
            MutationError::InvalidRequest(err) => {
                ErrorResponse::from(err).with_status_code(StatusCode::BAD_REQUEST)
            }
            MutationError::UnprocessableContent(err) => {
                ErrorResponse::from(err).with_status_code(StatusCode::UNPROCESSABLE_ENTITY)
            }
            MutationError::UnsupportedOperation(err) => {
                ErrorResponse::from(err).with_status_code(StatusCode::NOT_IMPLEMENTED)
            }
            MutationError::Conflict(err) => {
                ErrorResponse::from(err).with_status_code(StatusCode::CONFLICT)
            }
            MutationError::ConstraintNotMet(err) => {
                ErrorResponse::from(err).with_status_code(StatusCode::FORBIDDEN)
            }
        }
    }
}
