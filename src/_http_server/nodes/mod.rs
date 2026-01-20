//! HTTP server graph nodes for handling HTTP requests and responses.
//!
//! This module provides graph nodes for HTTP server integration, enabling
//! StreamWeave graphs to handle HTTP requests and produce HTTP responses.
//! It includes producers for generating HTTP requests, consumers for handling
//! responses, and transformers for routing and processing HTTP data.
//!
//! # Overview
//!
//! The HTTP server nodes enable building HTTP APIs using StreamWeave graphs.
//! They integrate Axum HTTP requests/responses with StreamWeave's graph-based
//! execution model, allowing complex request processing pipelines to be expressed
//! as graphs.
//!
//! # Key Concepts
//!
//! - **HTTP Request Production**: Producer nodes generate HTTP requests from
//!   graph data
//! - **HTTP Response Consumption**: Consumer nodes handle HTTP responses and
//!   correlate them with requests
//! - **Path Routing**: Transformer nodes route requests based on URL paths
//! - **Request-Response Correlation**: Consumer nodes match responses to
//!   original requests
//!
//! # Core Types
//!
//! - **[`HttpRequestProducer`]**: Producer node that generates HTTP requests
//! - **[`HttpResponseConsumer`]**: Consumer node that handles HTTP responses
//! - **[`HttpServerConsumerNode`]**: Dedicated graph node for HTTP response consumption
//! - **[`HttpResponseCorrelationConsumer`]**: Consumer that correlates responses
//!   with requests
//! - **[`crate::http_server::nodes::PathRouterTransformer`]**: Transformer that routes requests based on
//!   URL paths
//!
//! # Quick Start
//!
//! ## Basic HTTP Server Graph
//!
//! ```rust,no_run
//! use streamweave::{Graph, GraphBuilder};
//! use streamweave::http_server::nodes::*;
//!
//! // Create a graph with HTTP request producer and response consumer
//! let graph = GraphBuilder::new()
//!     .add_node("request_producer", HttpRequestProducer::new())
//!     .add_node("response_consumer", HttpResponseConsumer::new())
//!     .connect("request_producer", 0, "response_consumer", 0)
//!     .build();
//! ```
//!
//! ## Path-Based Routing
//!
//! ```rust,no_run
//! use streamweave::http_server::nodes::PathRouterTransformer;
//!
//! // Create a path router that routes to different handlers based on URL
//! let router = PathRouterTransformer::new()
//!     .route("/api/v1/users", "users_handler")
//!     .route("/api/v1/posts", "posts_handler");
//! ```
//!
//! # Integration with StreamWeave
//!
//! All nodes in this module implement the standard StreamWeave node traits
//! and can be used in any StreamWeave graph. They work seamlessly with other
//! graph nodes and support the standard error handling and configuration
//! mechanisms.

pub mod consumer;
pub mod consumer_node;
pub mod path_router_node;
pub mod path_router_transformer;
pub mod producer;
pub mod producer_node;

pub use consumer::*;
pub use consumer_node::*;
pub use path_router_node::*;
pub use path_router_transformer::*;
pub use producer::*;
pub use producer_node::*;

// Type alias for rustdoc links
/// Alias for [`PathBasedRouterTransformer`] used in documentation.
pub type PathRouterTransformer = PathBasedRouterTransformer;
