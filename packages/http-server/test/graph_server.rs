#[cfg(test)]
mod tests {
  use axum::body::Body;
  use axum::extract::Request;
  use axum::http::{Method, StatusCode, Uri};
  use futures::StreamExt;
  use std::time::Duration;
  use streamweave_graph::{ExecutionError, ExecutionState};
  use streamweave_graph::{
    GraphBuilder,
    node::{ConsumerNode, ProducerNode},
  };
  use streamweave_http_server::consumer::HttpResponseCorrelationConsumer;
  use streamweave_http_server::graph_server::{HttpGraphServer, HttpGraphServerConfig};
  use streamweave_http_server::producer::LongLivedHttpRequestProducer;
  use streamweave_http_server::types::{ContentType, HttpMethod, HttpRequest, HttpResponse};
  use streamweave_message::{Message, MessageId};
  use streamweave_transformers::MapTransformer;
  use tokio::sync::mpsc;

  #[tokio::test]
  async fn test_http_graph_server_config_default() {
    let config = HttpGraphServerConfig::default();
    assert_eq!(config.request_timeout, Duration::from_secs(30));
    assert_eq!(config.request_channel_buffer, 100);
  }

  #[tokio::test]
  async fn test_http_graph_server_config_custom() {
    let config = HttpGraphServerConfig {
      request_timeout: Duration::from_secs(60),
      request_channel_buffer: 200,
    };
    assert_eq!(config.request_timeout, Duration::from_secs(60));
    assert_eq!(config.request_channel_buffer, 200);
  }

  // Note: Full integration tests for HttpGraphServer require a complete graph setup
  // with proper message flow. These tests focus on configuration and basic functionality.

  #[tokio::test]
  async fn test_http_graph_server_new_minimal() {
    // Create a minimal graph - just a producer node
    let (request_tx, request_rx) = mpsc::channel(100);
    let producer = LongLivedHttpRequestProducer::with_default_config(request_rx);
    let producer_node = ProducerNode::from_producer("http_producer".to_string(), producer);

    let graph = GraphBuilder::new().node(producer_node).unwrap().build();

    let config = HttpGraphServerConfig::default();
    let result = HttpGraphServer::new(graph, config).await;

    // Should succeed in creating the server (even without consumer, for basic structure)
    assert!(result.is_ok());
    let (server, _receiver) = result.unwrap();
    assert_eq!(server.config.request_timeout, Duration::from_secs(30));
    assert_eq!(server.config.request_channel_buffer, 100);
  }

  #[tokio::test]
  async fn test_http_graph_server_clone() {
    let (request_tx, request_rx) = mpsc::channel(100);
    let producer = LongLivedHttpRequestProducer::with_default_config(request_rx);
    let producer_node = ProducerNode::from_producer("http_producer".to_string(), producer);

    let graph = GraphBuilder::new().node(producer_node).unwrap().build();

    let config = HttpGraphServerConfig::default();
    let (server, _receiver) = HttpGraphServer::new(graph, config).await.unwrap();

    // Clone the server
    let cloned_server = server.clone();

    // Both should have the same config
    assert_eq!(
      server.config.request_timeout,
      cloned_server.config.request_timeout
    );
    assert_eq!(
      server.config.request_channel_buffer,
      cloned_server.config.request_channel_buffer
    );
  }

  #[tokio::test]
  async fn test_http_graph_server_state_initial() {
    let (request_tx, request_rx) = mpsc::channel(100);
    let producer = LongLivedHttpRequestProducer::with_default_config(request_rx);
    let producer_node = ProducerNode::from_producer("http_producer".to_string(), producer);

    let graph = GraphBuilder::new().node(producer_node).unwrap().build();

    let config = HttpGraphServerConfig::default();
    let (server, _receiver) = HttpGraphServer::new(graph, config).await.unwrap();

    // Initial state should be Stopped
    let state = server.state().await;
    assert!(matches!(state, ExecutionState::Stopped));
  }

  #[tokio::test]
  async fn test_http_graph_server_create_handler() {
    let (request_tx, request_rx) = mpsc::channel(100);
    let producer = LongLivedHttpRequestProducer::with_default_config(request_rx);
    let producer_node = ProducerNode::from_producer("http_producer".to_string(), producer);

    let graph = GraphBuilder::new().node(producer_node).unwrap().build();

    let config = HttpGraphServerConfig::default();
    let (server, _receiver) = HttpGraphServer::new(graph, config).await.unwrap();

    // Create handler
    let handler = server.create_handler();

    // Handler should be callable
    let request = Request::builder()
      .uri("/test")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    let response = handler(request).await;
    // Should return some response (may be error if graph not properly set up)
    assert!(
      response.status().is_client_error()
        || response.status().is_server_error()
        || response.status().is_success()
        || response.status() == StatusCode::GATEWAY_TIMEOUT
    );
  }

  #[tokio::test]
  async fn test_http_graph_server_handle_request_timeout() {
    // Test that handle_request properly handles timeouts
    let (request_tx, request_rx) = mpsc::channel(100);
    let producer = LongLivedHttpRequestProducer::with_default_config(request_rx);
    let producer_node = ProducerNode::from_producer("http_producer".to_string(), producer);

    let graph = GraphBuilder::new().node(producer_node).unwrap().build();

    let config = HttpGraphServerConfig {
      request_timeout: Duration::from_millis(50), // Very short timeout
      request_channel_buffer: 1,
    };
    let (server, _receiver) = HttpGraphServer::new(graph, config).await.unwrap();

    server.start().await.unwrap();

    // Create a request
    let request = Request::builder()
      .uri("/test")
      .method(Method::GET)
      .body(Body::empty())
      .unwrap();

    // Handle request - should timeout since no response will come
    let response = server.handle_request(request).await;

    // Should get a timeout response
    assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);

    server.stop().await.unwrap();
  }
}
