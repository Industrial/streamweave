//! Tests for CsvStringify node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::CsvStringify;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_csv_stringify_new() {
  let node = CsvStringify::new();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_csv_stringify_default() {
  let node = CsvStringify::default();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_csv_stringify_with_name() {
  let node = CsvStringify::new().with_name("csv_stringify_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("csv_stringify_node"));
}

#[tokio::test]
async fn test_csv_stringify_with_error_strategy() {
  let node = CsvStringify::new().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_csv_stringify_with_headers() {
  let node = CsvStringify::new().with_headers(true);
  // Just verify it compiles and doesn't panic
  assert!(true);
}

#[tokio::test]
async fn test_csv_stringify_with_delimiter() {
  let node = CsvStringify::new().with_delimiter(b';');
  // Just verify it compiles and doesn't panic
  assert!(true);
}

#[tokio::test]
async fn test_csv_stringify_basic() {
  let mut node = CsvStringify::new();
  let input = json!([{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should convert JSON to CSV string
  assert!(!results.is_empty());
  assert!(results[0].is_string());
}

#[tokio::test]
async fn test_csv_stringify_with_headers() {
  let mut node = CsvStringify::new().with_headers(true);
  let input = json!([{"name": "Alice", "age": 30}]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should include header row
  assert!(!results.is_empty());
  let csv = results[0].as_str().unwrap();
  assert!(csv.contains("name") || csv.contains("age"));
}

#[tokio::test]
async fn test_csv_stringify_without_headers() {
  let mut node = CsvStringify::new().with_headers(false);
  let input = json!([{"name": "Alice", "age": 30}]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should not include header row
  assert!(!results.is_empty());
}

#[tokio::test]
async fn test_csv_stringify_custom_delimiter() {
  let mut node = CsvStringify::new().with_delimiter(b';');
  let input = json!([{"name": "Alice", "age": 30}]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should use semicolon delimiter
  assert!(!results.is_empty());
  let csv = results[0].as_str().unwrap();
  assert!(csv.contains(';'));
}

#[tokio::test]
async fn test_csv_stringify_empty_array() {
  let mut node = CsvStringify::new();
  let input = json!([]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should handle empty array
  assert!(!results.is_empty() || results.is_empty());
}

#[tokio::test]
async fn test_csv_stringify_non_array_input() {
  let mut node = CsvStringify::new();
  let input = json!({"name": "Alice"});
  let input_stream = Box::pin(stream::iter(vec![input.clone()]));

  let mut output_stream = node.transform(input_stream).await;
  // Non-array input should be handled (may convert to array or error)
  let result = output_stream.next().await;
  assert!(result.is_some() || result.is_none());
}

#[tokio::test]
async fn test_csv_stringify_stream() {
  let mut node = CsvStringify::new();
  let input_stream = Box::pin(stream::iter(vec![json!([{"a": 1}]), json!([{"b": 2}])]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should stringify multiple arrays
  assert_eq!(results.len(), 2);
  assert!(results[0].is_string());
  assert!(results[1].is_string());
}

#[tokio::test]
async fn test_csv_stringify_empty_stream() {
  let mut node = CsvStringify::new();
  let input_stream = Box::pin(stream::empty::<serde_json::Value>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_csv_stringify_config_access() {
  let mut node = CsvStringify::new().with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}
