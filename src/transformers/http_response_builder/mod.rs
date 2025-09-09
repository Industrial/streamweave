//! HTTP Response Builder Transformer
//!
//! This module provides a transformer that converts `ResponseData` into proper
//! `StreamWeaveHttpResponse` objects. It handles different content types, status codes,
//! headers, and streaming responses.
//!
//! # Overview
//!
//! The HTTP Response Builder Transformer is designed to be the central component for
//! building HTTP responses in StreamWeave. It provides:
//!
//! - **Type Safety**: Uses `ResponseData` enum to ensure proper response construction
//! - **Consistency**: All responses follow the same pattern and include standard headers
//! - **Streaming Support**: Handles both static and streaming responses
//! - **Configurability**: Supports custom headers, CORS, security headers, and more
//! - **Error Handling**: Converts errors into proper HTTP error responses
//!
//! # Usage
//!
//! ```rust
//! use streamweave::transformers::http_response_builder::{
//!     HttpResponseBuilderTransformer, ResponseData
//! };
//! use bytes::Bytes;
//! use http::StatusCode;
//! use futures::stream;
//! use streamweave::transformer::Transformer;
//! use futures::StreamExt;
//!
//! # async fn example() {
//! // Create a transformer with custom configuration
//! let mut transformer = HttpResponseBuilderTransformer::new()
//!     .with_default_content_type("application/json".to_string())
//!     .with_compression_enabled(true)
//!     .with_security_headers(true);
//!
//! // Create different types of responses
//! let success_response = ResponseData::ok(Bytes::from("{\"message\": \"Hello\"}"));
//! let error_response = ResponseData::error(StatusCode::NOT_FOUND, "Not found".to_string());
//!
//! // Process responses through the transformer
//! let input_stream = stream::iter(vec![success_response, error_response]);
//! let mut output_stream = transformer.transform(Box::pin(input_stream));
//!
//! while let Some(response) = output_stream.next().await {
//!     // Process the StreamWeaveHttpResponse
//!     println!("Status: {}, Body size: {}", response.status, response.body.len());
//! }
//! # }
//! ```
//!
//! # ResponseData Types
//!
//! The `ResponseData` enum supports two types of responses:
//!
//! 1. **Success**: Standard HTTP responses with status, body, and headers
//! 2. **Error**: Error responses with status and message (automatically formatted as JSON)
//!
//! # Configuration
//!
//! The transformer can be configured with:
//!
//! - Default content type
//! - Compression settings
//! - Default headers
//! - Security headers (X-Content-Type-Options, X-Frame-Options, etc.)
//! - CORS headers
//! - Custom CORS configuration
//!
//! # Integration
//!
//! This transformer integrates with other StreamWeave components:
//!
//! - **HTTP Router Transformer**: Routes requests and produces `ResponseData`
//! - **HTTP Middleware**: Processes `ResponseData` before building final responses
//! - **HTTP Response Consumer**: Consumes `StreamWeaveHttpResponse` to send to clients
//!
//! # Examples
//!
//! See the examples directory for complete usage examples demonstrating:
//! - Basic response building
//! - Error handling
//! - Middleware integration
//! - CORS and security headers

pub mod builder_utils;
pub mod response_data;
pub mod transformer;

// Re-export the main types for convenience
pub use builder_utils::{
  ErrorResponseBuilder, HtmlResponseBuilder, JsonResponseBuilder, TextResponseBuilder, responses,
};
pub use response_data::ResponseData;
pub use transformer::{
  CorsConfig, HttpResponseBuilderConfig, HttpResponseBuilderTransformer, ResponseBuilderError,
};

#[cfg(test)]
mod tests {
  use super::*;
  use crate::transformer::Transformer;
  use bytes::Bytes;
  use futures::StreamExt;
  use futures::stream;
  use http::StatusCode;

  #[tokio::test]
  async fn test_module_integration() {
    // Test that all components work together
    let mut transformer = HttpResponseBuilderTransformer::new()
      .with_default_content_type("application/json".to_string())
      .with_security_headers(true);

    let response_data = ResponseData::ok(Bytes::from("{\"test\": \"value\"}"));
    let input_stream = stream::iter(vec![response_data]);

    let mut output_stream = transformer.transform(Box::pin(input_stream));
    let mut responses = Vec::new();

    while let Some(response) = output_stream.next().await {
      responses.push(response);
    }

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].status, StatusCode::OK);
    assert!(responses[0].headers.contains_key("content-type"));
    assert!(responses[0].headers.contains_key("x-content-type-options"));
  }

  #[tokio::test]
  async fn test_error_response_integration() {
    let mut transformer = HttpResponseBuilderTransformer::new();
    let response_data = ResponseData::error(StatusCode::BAD_REQUEST, "Invalid request".to_string());

    let input_stream = stream::iter(vec![response_data]);
    let mut output_stream = transformer.transform(Box::pin(input_stream));
    let mut responses = Vec::new();

    while let Some(response) = output_stream.next().await {
      responses.push(response);
    }

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&responses[0].body).contains("Invalid request"));
  }

  #[test]
  fn test_response_data_convenience_methods() {
    // Test all convenience methods
    let ok_response = ResponseData::ok(Bytes::from("OK"));
    assert!(ok_response.is_success());
    assert_eq!(ok_response.status(), StatusCode::OK);

    let not_found = ResponseData::not_found("Not found".to_string());
    assert!(not_found.is_error());
    assert_eq!(not_found.status(), StatusCode::NOT_FOUND);

    let internal_error = ResponseData::internal_server_error("Server error".to_string());
    assert!(internal_error.is_error());
    assert_eq!(internal_error.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let bad_request = ResponseData::bad_request("Bad request".to_string());
    assert!(bad_request.is_error());
    assert_eq!(bad_request.status(), StatusCode::BAD_REQUEST);

    let unauthorized = ResponseData::unauthorized("Unauthorized".to_string());
    assert!(unauthorized.is_error());
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let forbidden = ResponseData::forbidden("Forbidden".to_string());
    assert!(forbidden.is_error());
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
  }

  #[test]
  fn test_cors_config() {
    let cors_config = CorsConfig {
      allowed_origins: vec!["https://example.com".to_string()],
      allowed_methods: vec!["GET".to_string(), "POST".to_string()],
      allowed_headers: vec!["content-type".to_string()],
      exposed_headers: vec!["x-custom".to_string()],
      allow_credentials: true,
      max_age: Some(3600),
    };

    assert_eq!(cors_config.allowed_origins.len(), 1);
    assert_eq!(cors_config.allowed_methods.len(), 2);
    assert!(cors_config.allow_credentials);
    assert_eq!(cors_config.max_age, Some(3600));
  }

  #[test]
  fn test_http_response_builder_config() {
    let mut default_headers = std::collections::HashMap::new();
    default_headers.insert("x-api-version".to_string(), "v1".to_string());

    let config = HttpResponseBuilderConfig {
      default_content_type: "text/html".to_string(),
      compression_enabled: true,
      default_headers: default_headers.clone(),
      add_security_headers: true,
      add_cors_headers: true,
      cors_config: None,
    };

    assert_eq!(config.default_content_type, "text/html");
    assert!(config.compression_enabled);
    assert_eq!(config.default_headers, default_headers);
    assert!(config.add_security_headers);
    assert!(config.add_cors_headers);
  }
}
