//! Tests for ArrayConcat node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::ArrayConcat;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_array_concat_new() {
  let node = ArrayConcat::new();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_array_concat_default() {
  let node = ArrayConcat::default();
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_array_concat_with_name() {
  let node = ArrayConcat::new().with_name("concat_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("concat_node"));
}

#[tokio::test]
async fn test_array_concat_with_error_strategy() {
  let node = ArrayConcat::new().with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_array_concat_basic() {
  let mut node = ArrayConcat::new();
  let input = json!([[1, 2], [3, 4]]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  let output = result.unwrap();
  assert_eq!(output, json!([1, 2, 3, 4]));
}

#[tokio::test]
async fn test_array_concat_multiple_arrays() {
  let mut node = ArrayConcat::new();
  let input = json!([[1, 2], [3, 4], [5, 6]]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  let output = result.unwrap();
  assert_eq!(output, json!([1, 2, 3, 4, 5, 6]));
}

#[tokio::test]
async fn test_array_concat_empty_array() {
  let mut node = ArrayConcat::new();
  let input = json!([]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  let output = result.unwrap();
  assert_eq!(output, json!([]));
}

#[tokio::test]
async fn test_array_concat_nested_empty() {
  let mut node = ArrayConcat::new();
  let input = json!([[], [1, 2], []]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  let output = result.unwrap();
  assert_eq!(output, json!([1, 2]));
}

#[tokio::test]
async fn test_array_concat_non_array_input() {
  let mut node = ArrayConcat::new();
  let input = json!(42);
  let input_stream = Box::pin(stream::iter(vec![input.clone()]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  // Non-array input should pass through unchanged
  assert!(result.is_some());
  assert_eq!(result.unwrap(), input);
}

#[tokio::test]
async fn test_array_concat_non_array_elements() {
  let mut node = ArrayConcat::new();
  let input = json!([[1, 2], "not an array", [3, 4]]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  let output = result.unwrap();
  // Only array elements should be concatenated
  assert_eq!(output, json!([1, 2, 3, 4]));
}

#[tokio::test]
async fn test_array_concat_stream() {
  let mut node = ArrayConcat::new();
  let input_stream = Box::pin(stream::iter(vec![
    json!([[1, 2]]),
    json!([[3, 4]]),
    json!([[5, 6]]),
  ]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 3);
  assert_eq!(results[0], json!([1, 2]));
  assert_eq!(results[1], json!([3, 4]));
  assert_eq!(results[2], json!([5, 6]));
}

#[tokio::test]
async fn test_array_concat_empty_stream() {
  let mut node = ArrayConcat::new();
  let input_stream = Box::pin(stream::empty::<serde_json::Value>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_array_concat_config_access() {
  let mut node = ArrayConcat::new().with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}
