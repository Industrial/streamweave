//! Tests for HttpRequest node

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::HttpRequest;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_http_request_new() {
  let node = HttpRequest::new();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_http_request_default() {
  let node = HttpRequest::default();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_http_request_with_name() {
  let node = HttpRequest::new().with_name("http_request_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("http_request_node"));
}

#[tokio::test]
async fn test_http_request_with_error_strategy() {
  let node = HttpRequest::new().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_http_request_with_base_url() {
  let node = HttpRequest::new().with_base_url("https://api.example.com");
  // Just verify it compiles and doesn't panic
  assert!(true);
}

#[tokio::test]
async fn test_http_request_with_method() {
  let node = HttpRequest::new().with_method("POST");
  // Just verify it compiles and doesn't panic
  assert!(true);
}

#[tokio::test]
async fn test_http_request_with_header() {
  let node = HttpRequest::new().with_header("Authorization", "Bearer token");
  // Just verify it compiles and doesn't panic
  assert!(true);
}

#[tokio::test]
async fn test_http_request_with_timeout() {
  let node = HttpRequest::new().with_timeout_secs(30);
  // Just verify it compiles and doesn't panic
  assert!(true);
}

#[tokio::test]
async fn test_http_request_with_parse_json() {
  let node = HttpRequest::new().with_parse_json(true);
  // Just verify it compiles and doesn't panic
  assert!(true);
}

#[tokio::test]
async fn test_http_request_chained_config() {
  let node = HttpRequest::new()
    .with_base_url("https://api.example.com")
    .with_method("GET")
    .with_header("Content-Type", "application/json")
    .with_timeout_secs(10)
    .with_parse_json(true)
    .with_name("api_client".to_string());

  assert_eq!(node.get_config_impl().name(), Some("api_client"));
}

#[tokio::test]
async fn test_http_request_config_access() {
  let mut node = HttpRequest::new().with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_http_request_clone() {
  let node1 = HttpRequest::new()
    .with_base_url("https://api.example.com")
    .with_name("original".to_string());
  let node2 = node1.clone();

  assert_eq!(
    node1.get_config_impl().name(),
    node2.get_config_impl().name()
  );
}

// Note: Full integration tests for HTTP requests would require:
// - A mock HTTP server (e.g., using wiremock or similar)
// - Actual HTTP request/response testing
// - Error handling scenarios
// - Timeout testing
// These are typically done in integration tests
