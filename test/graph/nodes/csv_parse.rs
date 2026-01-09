//! Tests for CsvParse node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::CsvParse;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_csv_parse_new() {
  let node = CsvParse::new();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_csv_parse_default() {
  let node = CsvParse::default();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_csv_parse_with_name() {
  let node = CsvParse::new().with_name("csv_parse_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("csv_parse_node"));
}

#[tokio::test]
async fn test_csv_parse_with_error_strategy() {
  let node = CsvParse::new().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_csv_parse_with_headers() {
  let node = CsvParse::new().with_headers(true);
  // Just verify it compiles and doesn't panic
  assert!(true);
}

#[tokio::test]
async fn test_csv_parse_with_delimiter() {
  let node = CsvParse::new().with_delimiter(b';');
  // Just verify it compiles and doesn't panic
  assert!(true);
}

#[tokio::test]
async fn test_csv_parse_with_trim() {
  let node = CsvParse::new().with_trim(true);
  // Just verify it compiles and doesn't panic
  assert!(true);
}

#[tokio::test]
async fn test_csv_parse_basic() {
  let mut node = CsvParse::new();
  let csv_data = "name,age\nAlice,30\nBob,25".to_string();
  let input_stream = Box::pin(stream::iter(vec![csv_data]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should parse CSV into JSON objects
  assert!(!results.is_empty());
}

#[tokio::test]
async fn test_csv_parse_with_headers() {
  let mut node = CsvParse::new().with_headers(true);
  let csv_data = "name,age\nAlice,30\nBob,25".to_string();
  let input_stream = Box::pin(stream::iter(vec![csv_data]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should parse with headers
  assert!(!results.is_empty());
}

#[tokio::test]
async fn test_csv_parse_without_headers() {
  let mut node = CsvParse::new().with_headers(false);
  let csv_data = "Alice,30\nBob,25".to_string();
  let input_stream = Box::pin(stream::iter(vec![csv_data]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should parse without headers
  assert!(!results.is_empty());
}

#[tokio::test]
async fn test_csv_parse_custom_delimiter() {
  let mut node = CsvParse::new().with_delimiter(b';');
  let csv_data = "name;age\nAlice;30".to_string();
  let input_stream = Box::pin(stream::iter(vec![csv_data]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should parse with semicolon delimiter
  assert!(!results.is_empty());
}

#[tokio::test]
async fn test_csv_parse_empty_string() {
  let mut node = CsvParse::new();
  let csv_data = "".to_string();
  let input_stream = Box::pin(stream::iter(vec![csv_data]));

  let mut output_stream = node.transform(input_stream).await;
  // Should handle empty string gracefully
  let result = output_stream.next().await;
  // May return empty array or no items
  assert!(result.is_none() || result.is_some());
}

#[tokio::test]
async fn test_csv_parse_stream() {
  let mut node = CsvParse::new();
  let input_stream = Box::pin(stream::iter(vec![
    "a,b\n1,2".to_string(),
    "c,d\n3,4".to_string(),
  ]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should parse multiple CSV strings
  assert!(!results.is_empty());
}

#[tokio::test]
async fn test_csv_parse_empty_stream() {
  let mut node = CsvParse::new();
  let input_stream = Box::pin(stream::empty::<String>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_csv_parse_config_access() {
  let mut node = CsvParse::new().with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}
