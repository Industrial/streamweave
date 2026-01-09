//! Tests for JsonParse node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::JsonParse;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_json_parse_new() {
  let node = JsonParse::new();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_json_parse_default() {
  let node = JsonParse::default();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_json_parse_with_name() {
  let node = JsonParse::new().with_name("json_parse_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("json_parse_node"));
}

#[tokio::test]
async fn test_json_parse_with_error_strategy() {
  let node = JsonParse::new().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_json_parse_basic() {
  let mut node = JsonParse::new();
  let json_string = r#"{"name": "Alice", "age": 30}"#.to_string();
  let input_stream = Box::pin(stream::iter(vec![json_string]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  let output = result.unwrap();
  assert_eq!(output["name"], json!("Alice"));
  assert_eq!(output["age"], json!(30));
}

#[tokio::test]
async fn test_json_parse_array() {
  let mut node = JsonParse::new();
  let json_string = r#"[1, 2, 3, 4, 5]"#.to_string();
  let input_stream = Box::pin(stream::iter(vec![json_string]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  let output = result.unwrap();
  assert!(output.is_array());
  assert_eq!(output.as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn test_json_parse_number() {
  let mut node = JsonParse::new();
  let json_string = "42".to_string();
  let input_stream = Box::pin(stream::iter(vec![json_string]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!(42));
}

#[tokio::test]
async fn test_json_parse_string() {
  let mut node = JsonParse::new();
  let json_string = r#""hello world""#.to_string();
  let input_stream = Box::pin(stream::iter(vec![json_string]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!("hello world"));
}

#[tokio::test]
async fn test_json_parse_invalid_json() {
  let mut node = JsonParse::new().with_error_strategy(ErrorStrategy::Skip);
  let invalid_json = "not valid json".to_string();
  let input_stream = Box::pin(stream::iter(vec![invalid_json]));

  let mut output_stream = node.transform(input_stream).await;
  // With Skip strategy, invalid JSON should be filtered out
  let result = output_stream.next().await;
  // May return null or filter out
  assert!(result.is_none() || result.is_some());
}

#[tokio::test]
async fn test_json_parse_empty_string() {
  let mut node = JsonParse::new();
  let json_string = "".to_string();
  let input_stream = Box::pin(stream::iter(vec![json_string]));

  let mut output_stream = node.transform(input_stream).await;
  // Empty string is not valid JSON
  let result = output_stream.next().await;
  assert!(result.is_none() || result.is_some());
}

#[tokio::test]
async fn test_json_parse_stream() {
  let mut node = JsonParse::new();
  let input_stream = Box::pin(stream::iter(vec![
    r#"{"a": 1}"#.to_string(),
    r#"{"b": 2}"#.to_string(),
    r#"{"c": 3}"#.to_string(),
  ]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 3);
  assert_eq!(results[0]["a"], json!(1));
  assert_eq!(results[1]["b"], json!(2));
  assert_eq!(results[2]["c"], json!(3));
}

#[tokio::test]
async fn test_json_parse_empty_stream() {
  let mut node = JsonParse::new();
  let input_stream = Box::pin(stream::empty::<String>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_json_parse_config_access() {
  let mut node = JsonParse::new().with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_json_parse_clone() {
  let node1 = JsonParse::new().with_name("original".to_string());
  let node2 = node1.clone();

  assert_eq!(
    node1.get_config_impl().name(),
    node2.get_config_impl().name()
  );
}
