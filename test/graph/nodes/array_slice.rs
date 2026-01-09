//! Tests for ArraySlice node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::ArraySlice;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_array_slice_new() {
  let node = ArraySlice::new(0, Some(3));
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_array_slice_with_name() {
  let node = ArraySlice::new(0, Some(3)).with_name("slice_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("slice_node"));
}

#[tokio::test]
async fn test_array_slice_with_error_strategy() {
  let node = ArraySlice::new(0, Some(3)).with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_array_slice_basic() {
  let mut node = ArraySlice::new(1, Some(4));
  let input = json!([1, 2, 3, 4, 5]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should slice from index 1 to 4 (exclusive)
  assert_eq!(result.unwrap(), json!([2, 3, 4]));
}

#[tokio::test]
async fn test_array_slice_to_end() {
  let mut node = ArraySlice::new(2, None);
  let input = json!([1, 2, 3, 4, 5]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should slice from index 2 to end
  assert_eq!(result.unwrap(), json!([3, 4, 5]));
}

#[tokio::test]
async fn test_array_slice_from_start() {
  let mut node = ArraySlice::new(0, Some(3));
  let input = json!([1, 2, 3, 4, 5]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!([1, 2, 3]));
}

#[tokio::test]
async fn test_array_slice_empty_array() {
  let mut node = ArraySlice::new(0, Some(3));
  let input = json!([]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!([]));
}

#[tokio::test]
async fn test_array_slice_out_of_bounds() {
  let mut node = ArraySlice::new(10, Some(20));
  let input = json!([1, 2, 3]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should return empty array for out of bounds
  assert_eq!(result.unwrap(), json!([]));
}

#[tokio::test]
async fn test_array_slice_non_array_input() {
  let mut node = ArraySlice::new(0, Some(3));
  let input = json!(42);
  let input_stream = Box::pin(stream::iter(vec![input.clone()]));

  let mut output_stream = node.transform(input_stream).await;
  // Non-array input should pass through or return null
  let result = output_stream.next().await;
  assert!(result.is_some());
}

#[tokio::test]
async fn test_array_slice_stream() {
  let mut node = ArraySlice::new(1, Some(3));
  let input_stream = Box::pin(stream::iter(vec![json!([1, 2, 3, 4]), json!([5, 6, 7, 8])]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 2);
  assert_eq!(results[0], json!([2, 3]));
  assert_eq!(results[1], json!([6, 7]));
}

#[tokio::test]
async fn test_array_slice_empty_stream() {
  let mut node = ArraySlice::new(0, Some(3));
  let input_stream = Box::pin(stream::empty::<serde_json::Value>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_array_slice_config_access() {
  let mut node = ArraySlice::new(0, Some(3)).with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}
