//! HTTP Graph Server Setup
//!
//! This module sets up the complete HTTP Graph Server with all handlers
//! and demonstrates various graph patterns.

#[cfg(feature = "http-server")]
use crate::handlers;
#[cfg(feature = "http-server")]
use axum::{Router, routing::get, routing::post};
#[cfg(feature = "http-server")]
use streamweave::graph::{ConsumerNode, GraphBuilder, ProducerNode, TransformerNode};
#[cfg(feature = "http-server")]
use streamweave::http_server::consumer::HttpResponseCorrelationConsumer;
#[cfg(feature = "http-server")]
use streamweave::http_server::producer::{HttpRequestProducerConfig, LongLivedHttpRequestProducer};
#[cfg(feature = "http-server")]
use streamweave::http_server::{
  HttpGraphServer, HttpGraphServerConfig,
  transformers::{PathBasedRouterTransformer, PathRouterConfig, RoutePattern},
};
#[cfg(feature = "http-server")]
use tokio::sync::mpsc;

/// Creates and starts the HTTP Graph Server
#[cfg(feature = "http-server")]
pub async fn create_graph_server() -> Result<(), Box<dyn std::error::Error>> {
  // Create channel for HTTP requests
  let (_request_sender, request_receiver) = mpsc::channel(100);

  // Create HTTP request producer (injects requests into graph)
  let http_producer =
    LongLivedHttpRequestProducer::new(request_receiver, HttpRequestProducerConfig::default());

  // Create path-based router
  // Routes requests to different output ports based on URL path
  let router = PathBasedRouterTransformer::new(PathRouterConfig {
    routes: vec![
      RoutePattern {
        pattern: "/api/rest/*".to_string(),
        port: 0, // Route to REST handler
      },
      RoutePattern {
        pattern: "/api/graphql".to_string(),
        port: 1, // Route to GraphQL handler
      },
      RoutePattern {
        pattern: "/api/rpc/*".to_string(),
        port: 2, // Route to RPC handler
      },
      RoutePattern {
        pattern: "/static/*".to_string(),
        port: 3, // Route to Static file handler
      },
      RoutePattern {
        pattern: "/api/fanout".to_string(),
        port: 4, // Route to Fan-out example
      },
    ],
    default_port: Some(5), // Default/404 handler
  });

  // Create handler transformers inline
  // Each handler is a MapTransformer that processes Message<HttpRequest> -> Message<HttpResponse>
  // We inline them here to avoid type inference issues with impl Transformer return types
  use streamweave::http_server::HttpRequest;
  use streamweave::message::Message;
  use streamweave::transformers::map::map_transformer::MapTransformer;

  // REST handler - processes REST API requests
  let rest_handler =
    MapTransformer::new(move |msg: Message<HttpRequest>| handlers::handle_rest_request(msg));

  // GraphQL handler - processes GraphQL-style queries
  let graphql_handler =
    MapTransformer::new(move |msg: Message<HttpRequest>| handlers::handle_graphql_request(msg));

  // RPC handler - processes RPC-style requests
  let rpc_handler =
    MapTransformer::new(move |msg: Message<HttpRequest>| handlers::handle_rpc_request(msg));

  // Static file handler - serves static content
  let static_handler =
    MapTransformer::new(move |msg: Message<HttpRequest>| handlers::handle_static_request(msg));

  // Fan-out handler - demonstrates fan-out patterns
  let fanout_handler =
    MapTransformer::new(move |msg: Message<HttpRequest>| handlers::handle_fanout_request(msg));

  // Default handler - handles 404 and health checks
  let default_handler =
    MapTransformer::new(move |msg: Message<HttpRequest>| handlers::handle_default_request(msg));

  // Create response correlation consumer
  // This matches responses to their original requests using request_id
  let response_consumer =
    HttpResponseCorrelationConsumer::with_timeout(std::time::Duration::from_secs(30));

  // Build the graph
  // The graph structure:
  //   http_producer -> router -> [rest_handler, graphql_handler, rpc_handler, static_handler, fanout_handler, default_handler] -> response_consumer
  let graph = GraphBuilder::new()
    // Add HTTP request producer
    .add_node(
      "http_producer".to_string(),
      ProducerNode::from_producer("http_producer".to_string(), http_producer),
    )?
    // Add path-based router
    .add_node(
      "router".to_string(),
      TransformerNode::from_transformer("router".to_string(), router),
    )?
    // Add REST handler
    .add_node(
      "rest_handler".to_string(),
      TransformerNode::from_transformer("rest_handler".to_string(), rest_handler),
    )?
    // Add GraphQL handler
    .add_node(
      "graphql_handler".to_string(),
      TransformerNode::from_transformer("graphql_handler".to_string(), graphql_handler),
    )?
    // Add RPC handler
    .add_node(
      "rpc_handler".to_string(),
      TransformerNode::from_transformer("rpc_handler".to_string(), rpc_handler),
    )?
    // Add Static file handler
    .add_node(
      "static_handler".to_string(),
      TransformerNode::from_transformer("static_handler".to_string(), static_handler),
    )?
    // Add Fan-out handler
    .add_node(
      "fanout_handler".to_string(),
      TransformerNode::from_transformer("fanout_handler".to_string(), fanout_handler),
    )?
    // Add Default/404 handler
    .add_node(
      "default_handler".to_string(),
      TransformerNode::from_transformer("default_handler".to_string(), default_handler),
    )?
    // Add Response correlation consumer
    .add_node(
      "response_consumer".to_string(),
      ConsumerNode::from_consumer("response_consumer".to_string(), response_consumer),
    )?
    // Connect producer to router
    .connect_by_name("http_producer", "router")?
    // Connect router outputs to handlers
    // Router port 0 -> REST handler
    .connect_by_name("router:0", "rest_handler")?
    // Router port 1 -> GraphQL handler
    .connect_by_name("router:1", "graphql_handler")?
    // Router port 2 -> RPC handler
    .connect_by_name("router:2", "rpc_handler")?
    // Router port 3 -> Static handler
    .connect_by_name("router:3", "static_handler")?
    // Router port 4 -> Fan-out handler
    .connect_by_name("router:4", "fanout_handler")?
    // Router port 5 -> Default handler (404)
    .connect_by_name("router:5", "default_handler")?
    // Connect all handlers to response consumer
    .connect_by_name("rest_handler", "response_consumer")?
    .connect_by_name("graphql_handler", "response_consumer")?
    .connect_by_name("rpc_handler", "response_consumer")?
    .connect_by_name("static_handler", "response_consumer")?
    .connect_by_name("fanout_handler", "response_consumer")?
    .connect_by_name("default_handler", "response_consumer")?
    .build();

  // Create the HTTP Graph Server
  let (server, _request_receiver) = HttpGraphServer::new(
    graph,
    HttpGraphServerConfig {
      request_timeout: std::time::Duration::from_secs(30),
      request_channel_buffer: 100,
    },
  )
  .await?;

  // Start the graph executor
  server.start().await?;

  // Create Axum handler
  let handler = server.create_handler();

  // Build Axum router
  let app = Router::new()
    .route("/api/rest/*path", get(handler.clone()))
    .route("/api/rest/*path", post(handler.clone()))
    .route("/api/graphql", post(handler.clone()))
    .route("/api/rpc/*path", post(handler.clone()))
    .route("/static/*path", get(handler.clone()))
    .route("/api/fanout", get(handler.clone()))
    .route("/api/health", get(handler.clone()))
    .route("/*path", get(handler));

  // Start Axum server
  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
  println!("✅ Server started successfully!");
  println!();
  axum::serve(listener, app).await?;

  Ok(())
}
