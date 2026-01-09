//! Tests for ArrayContains node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::ArrayContains;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_array_contains_new() {
  let node = ArrayContains::new(json!(5));
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_array_contains_with_name() {
  let node = ArrayContains::new(json!(5)).with_name("contains_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("contains_node"));
}

#[tokio::test]
async fn test_array_contains_with_error_strategy() {
  let node = ArrayContains::new(json!(5)).with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_array_contains_basic() {
  let mut node = ArrayContains::new(json!(3));
  let input = json!([1, 2, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should pass through if contains
  assert_eq!(result.unwrap(), json!([1, 2, 3, 4]));
}

#[tokio::test]
async fn test_array_contains_not_found() {
  let mut node = ArrayContains::new(json!(99));
  let input = json!([1, 2, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  // Should filter out arrays that don't contain the value
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_array_contains_empty_array() {
  let mut node = ArrayContains::new(json!(1));
  let input = json!([]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  // Empty array doesn't contain anything
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_array_contains_non_array_input() {
  let mut node = ArrayContains::new(json!(1));
  let input = json!(42);
  let input_stream = Box::pin(stream::iter(vec![input.clone()]));

  let mut output_stream = node.transform(input_stream).await;
  // Non-array input should be filtered out
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_array_contains_string_value() {
  let mut node = ArrayContains::new(json!("test"));
  let input = json!(["hello", "test", "world"]);
  let input_stream = Box::pin(stream::iter(vec![input.clone()]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), input);
}

#[tokio::test]
async fn test_array_contains_stream() {
  let mut node = ArrayContains::new(json!(2));
  let input_stream = Box::pin(stream::iter(vec![
    json!([1, 2, 3]),
    json!([4, 5, 6]),
    json!([2, 7, 8]),
  ]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should only include arrays containing 2
  assert_eq!(results.len(), 2);
  assert_eq!(results[0], json!([1, 2, 3]));
  assert_eq!(results[1], json!([2, 7, 8]));
}

#[tokio::test]
async fn test_array_contains_empty_stream() {
  let mut node = ArrayContains::new(json!(1));
  let input_stream = Box::pin(stream::empty::<serde_json::Value>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_array_contains_config_access() {
  let mut node = ArrayContains::new(json!(1)).with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}
