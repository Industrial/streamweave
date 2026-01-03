//! # HTTP Error Handling
//!
//! This module provides comprehensive error handling for HTTP server integration,
//! mapping StreamWeave errors to appropriate HTTP status codes and formatting
//! error responses as JSON.
//!
//! ## Features
//!
//! - Maps StreamWeave errors to HTTP status codes
//! - Formats error responses as structured JSON
//! - Supports different error types (validation, not found, server errors)
//! - Supports custom error responses
//! - Preserves error details in development mode
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::http_server::error::{map_to_http_error, ErrorResponse};
//! use streamweave::error::StreamError;
//! use axum::http::StatusCode;
//!
//! let stream_error = StreamError::new(
//!     Box::new(std::io::Error::other("Validation failed")),
//!     // ... error context ...
//! );
//!
//! let http_error = map_to_http_error(&stream_error, false);
//! assert_eq!(http_error.status, StatusCode::BAD_REQUEST);
//! ```

#[cfg(feature = "http-server")]
use axum::http::StatusCode;
#[cfg(feature = "http-server")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "http-server")]
use std::fmt;
#[cfg(feature = "http-server")]
use streamweave::error::StreamError;

/// Structured error response for HTTP endpoints.
///
/// This type represents a standardized error response format that can be
/// serialized to JSON and sent to clients.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::error::ErrorResponse;
/// use axum::http::StatusCode;
///
/// let error = ErrorResponse {
///     status: StatusCode::BAD_REQUEST,
///     message: "Invalid input".to_string(),
///     error: Some("ValidationError".to_string()),
///     details: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "http-server")]
pub struct ErrorResponse {
  /// HTTP status code for the error
  pub status: u16,
  /// Human-readable error message
  pub message: String,
  /// Error type identifier (e.g., "ValidationError", "NotFoundError")
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
  /// Additional error details (only included in development mode)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub details: Option<ErrorDetails>,
}

/// Detailed error information (only included in development mode).
///
/// This contains additional context about the error that may be useful
/// for debugging but should not be exposed in production.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "http-server")]
pub struct ErrorDetails {
  /// The underlying error message
  pub error: String,
  /// Component name where the error occurred
  #[serde(skip_serializing_if = "Option::is_none")]
  pub component: Option<String>,
  /// Timestamp when the error occurred
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timestamp: Option<String>,
  /// Additional context about the error
  #[serde(skip_serializing_if = "Option::is_none")]
  pub context: Option<String>,
}

/// Maps a StreamWeave `StreamError` to an HTTP status code.
///
/// This function analyzes the error and determines the most appropriate
/// HTTP status code based on the error type and message.
///
/// ## Error Type Mapping
///
/// - **Validation errors** (e.g., "invalid", "missing", "required") → `400 Bad Request`
/// - **Not found errors** (e.g., "not found", "missing") → `404 Not Found`
/// - **Authentication errors** (e.g., "unauthorized", "forbidden") → `401 Unauthorized` or `403 Forbidden`
/// - **Rate limiting errors** (e.g., "rate limit", "too many") → `429 Too Many Requests`
/// - **Server errors** (default) → `500 Internal Server Error`
///
/// ## Arguments
///
/// * `error` - The StreamWeave error to map
/// * `include_details` - Whether to include detailed error information (development mode)
///
/// ## Returns
///
/// An `ErrorResponse` with the appropriate status code and formatted message.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::error::map_to_http_error;
/// use streamweave::error::{StreamError, ErrorContext, ComponentInfo};
/// use axum::http::StatusCode;
///
/// let stream_error = StreamError::new(
///     Box::new(std::io::Error::other("Invalid input: missing required field")),
///     ErrorContext {
///         timestamp: chrono::Utc::now(),
///         item: None,
///         component_name: "validator".to_string(),
///         component_type: "ValidatorTransformer".to_string(),
///     },
///     ComponentInfo {
///         name: "validator".to_string(),
///         type_name: "ValidatorTransformer".to_string(),
///     },
/// );
///
/// let http_error = map_to_http_error(&stream_error, false);
/// assert_eq!(http_error.status, StatusCode::BAD_REQUEST.as_u16());
/// ```
#[cfg(feature = "http-server")]
pub fn map_to_http_error<T>(error: &StreamError<T>, include_details: bool) -> ErrorResponse
where
  T: fmt::Debug + Clone + Send + Sync,
{
  let error_message = error.to_string();
  let error_lower = error_message.to_lowercase();

  // Determine status code based on error message patterns
  let (status, error_type) = if error_lower.contains("not found")
    || error_lower.contains("missing") && error_lower.contains("not found")
  {
    (StatusCode::NOT_FOUND, Some("NotFoundError".to_string()))
  } else if error_lower.contains("unauthorized") || error_lower.contains("authentication") {
    (
      StatusCode::UNAUTHORIZED,
      Some("UnauthorizedError".to_string()),
    )
  } else if error_lower.contains("forbidden") || error_lower.contains("permission") {
    (StatusCode::FORBIDDEN, Some("ForbiddenError".to_string()))
  } else if error_lower.contains("rate limit") || error_lower.contains("too many requests") {
    (
      StatusCode::TOO_MANY_REQUESTS,
      Some("RateLimitError".to_string()),
    )
  } else if error_lower.contains("invalid")
    || error_lower.contains("validation")
    || error_lower.contains("bad request")
    || error_lower.contains("malformed")
  {
    (StatusCode::BAD_REQUEST, Some("ValidationError".to_string()))
  } else if error_lower.contains("conflict") || error_lower.contains("already exists") {
    (StatusCode::CONFLICT, Some("ConflictError".to_string()))
  } else if error_lower.contains("timeout") || error_lower.contains("timed out") {
    (
      StatusCode::REQUEST_TIMEOUT,
      Some("TimeoutError".to_string()),
    )
  } else if error_lower.contains("not implemented") {
    (
      StatusCode::NOT_IMPLEMENTED,
      Some("NotImplementedError".to_string()),
    )
  } else if error_lower.contains("service unavailable") || error_lower.contains("unavailable") {
    (
      StatusCode::SERVICE_UNAVAILABLE,
      Some("ServiceUnavailableError".to_string()),
    )
  } else {
    // Default to internal server error
    (
      StatusCode::INTERNAL_SERVER_ERROR,
      Some("InternalServerError".to_string()),
    )
  };

  // Build error details if in development mode
  let details = if include_details {
    Some(ErrorDetails {
      error: error_message.clone(),
      component: Some(error.context.component_name.clone()),
      timestamp: Some(error.context.timestamp.to_rfc3339()),
      context: Some(format!("{:?}", error.context)),
    })
  } else {
    None
  };

  ErrorResponse {
    status: status.as_u16(),
    message: error_message,
    error: error_type,
    details,
  }
}

/// Maps a generic error to an HTTP error response.
///
/// This is a convenience function for converting any error type to an
/// HTTP error response. It wraps the error in a `StreamError` and then
/// maps it to an HTTP status code.
///
/// ## Arguments
///
/// * `error` - The error to convert
/// * `status` - Optional HTTP status code (defaults to 500 if not provided)
/// * `include_details` - Whether to include detailed error information
///
/// ## Returns
///
/// An `ErrorResponse` with the appropriate status code and message.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::error::map_generic_error;
/// use axum::http::StatusCode;
///
/// let error = std::io::Error::other("Something went wrong");
/// let http_error = map_generic_error(&error, Some(StatusCode::BAD_REQUEST), false);
/// ```
#[cfg(feature = "http-server")]
pub fn map_generic_error(
  error: &dyn std::error::Error,
  status: Option<StatusCode>,
  include_details: bool,
) -> ErrorResponse {
  let status = status.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
  let message = error.to_string();

  let details = if include_details {
    Some(ErrorDetails {
      error: format!("{:?}", error),
      component: None,
      timestamp: Some(chrono::Utc::now().to_rfc3339()),
      context: None,
    })
  } else {
    None
  };

  ErrorResponse {
    status: status.as_u16(),
    message,
    error: Some("Error".to_string()),
    details,
  }
}

/// Creates a custom error response with a specific message and status code.
///
/// This function allows you to create custom error responses for specific
/// error conditions in your application.
///
/// ## Arguments
///
/// * `status` - HTTP status code
/// * `message` - Error message
/// * `error_type` - Optional error type identifier
/// * `include_details` - Whether to include detailed error information
///
/// ## Returns
///
/// An `ErrorResponse` with the specified status code and message.
///
/// ## Example
///
/// ```rust,no_run
/// use streamweave::http_server::error::create_custom_error;
/// use axum::http::StatusCode;
///
/// let error = create_custom_error(
///     StatusCode::BAD_REQUEST,
///     "Invalid user ID format",
///     Some("InvalidUserIdError".to_string()),
///     false,
/// );
/// ```
#[cfg(feature = "http-server")]
pub fn create_custom_error(
  status: StatusCode,
  message: impl Into<String>,
  error_type: Option<String>,
  include_details: bool,
) -> ErrorResponse {
  let message_str = message.into();
  ErrorResponse {
    status: status.as_u16(),
    message: message_str.clone(),
    error: error_type,
    details: if include_details {
      Some(ErrorDetails {
        error: message_str,
        component: None,
        timestamp: Some(chrono::Utc::now().to_rfc3339()),
        context: None,
      })
    } else {
      None
    },
  }
}

/// Determines if the application is running in development mode.
///
/// This checks the `RUST_ENV` or `ENVIRONMENT` environment variable
/// to determine if detailed error information should be included.
///
/// ## Returns
///
/// `true` if in development mode, `false` otherwise.
#[cfg(feature = "http-server")]
pub fn is_development_mode() -> bool {
  std::env::var("RUST_ENV")
    .or_else(|_| std::env::var("ENVIRONMENT"))
    .map(|env| env.to_lowercase() == "development" || env.to_lowercase() == "dev")
    .unwrap_or(false)
}
