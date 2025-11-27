//! # HTTP Server Support
//!
//! This module provides HTTP server capabilities for StreamWeave, enabling REST microservices
//! with streaming request/response bodies through StreamWeave pipelines.
//!
//! ## Features
//!
//! - HTTP request producer that converts incoming HTTP requests to stream items
//! - HTTP response consumer that converts stream items to HTTP responses
//! - Streaming request/response body support
//! - Axum integration for route handling and middleware
//! - Support for REST API patterns (GET, POST, PUT, DELETE, etc.)
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::http_server::{HttpRequest, HttpResponse};
//! use axum::{Router, routing::get};
//!
//! // Create an HTTP server with StreamWeave pipeline integration
//! let app = Router::new()
//!     .route("/api/data", get(handle_request));
//! ```
//!
//! ## Feature Flag
//!
//! This module requires the `http-server` feature flag to be enabled.

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub mod consumer;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub mod handler;
/// Input types for HTTP server integration.
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub mod input;
/// Output types for HTTP server integration.
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub mod output;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub mod producer;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub mod types;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub use consumer::{HttpResponseConsumer, HttpResponseConsumerConfig};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub use handler::{create_pipeline_handler, create_simple_handler};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub use producer::{HttpRequestProducer, HttpRequestProducerConfig};
#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub use types::{ContentType, HttpMethod, HttpRequest, HttpResponse};
