//! Tests for ArrayLength node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::ArrayLength;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_array_length_new() {
  let node = ArrayLength::new();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_array_length_default() {
  let node = ArrayLength::default();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_array_length_with_name() {
  let node = ArrayLength::new().with_name("length_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("length_node"));
}

#[tokio::test]
async fn test_array_length_with_error_strategy() {
  let node = ArrayLength::new().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_array_length_basic() {
  let mut node = ArrayLength::new();
  let input = json!([1, 2, 3, 4, 5]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!(5));
}

#[tokio::test]
async fn test_array_length_empty_array() {
  let mut node = ArrayLength::new();
  let input = json!([]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!(0));
}

#[tokio::test]
async fn test_array_length_single_element() {
  let mut node = ArrayLength::new();
  let input = json!([42]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!(1));
}

#[tokio::test]
async fn test_array_length_non_array_input() {
  let mut node = ArrayLength::new();
  let input = json!(42);
  let input_stream = Box::pin(stream::iter(vec![input.clone()]));

  let mut output_stream = node.transform(input_stream).await;
  // Non-array input should return null or 0
  let result = output_stream.next().await;
  assert!(result.is_some());
}

#[tokio::test]
async fn test_array_length_stream() {
  let mut node = ArrayLength::new();
  let input_stream = Box::pin(stream::iter(vec![
    json!([1, 2]),
    json!([3, 4, 5, 6]),
    json!([7]),
  ]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 3);
  assert_eq!(results[0], json!(2));
  assert_eq!(results[1], json!(4));
  assert_eq!(results[2], json!(1));
}

#[tokio::test]
async fn test_array_length_empty_stream() {
  let mut node = ArrayLength::new();
  let input_stream = Box::pin(stream::empty::<serde_json::Value>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_array_length_config_access() {
  let mut node = ArrayLength::new().with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}
