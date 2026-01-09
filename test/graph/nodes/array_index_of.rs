//! Tests for ArrayIndexOf node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::ArrayIndexOf;
use streamweave::transformers::ArrayIndexOfMode;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_array_index_of_new() {
  let node = ArrayIndexOf::new(json!(5), ArrayIndexOfMode::First);
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_array_index_of_with_name() {
  let node =
    ArrayIndexOf::new(json!(5), ArrayIndexOfMode::First).with_name("index_of_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("index_of_node"));
}

#[tokio::test]
async fn test_array_index_of_with_error_strategy() {
  let node =
    ArrayIndexOf::new(json!(5), ArrayIndexOfMode::First).with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_array_index_of_first() {
  let mut node = ArrayIndexOf::new(json!(3), ArrayIndexOfMode::First);
  let input = json!([1, 2, 3, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should return first occurrence index
  assert_eq!(result.unwrap(), json!(2));
}

#[tokio::test]
async fn test_array_index_of_last() {
  let mut node = ArrayIndexOf::new(json!(3), ArrayIndexOfMode::Last);
  let input = json!([1, 2, 3, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should return last occurrence index
  assert_eq!(result.unwrap(), json!(3));
}

#[tokio::test]
async fn test_array_index_of_not_found() {
  let mut node = ArrayIndexOf::new(json!(99), ArrayIndexOfMode::First);
  let input = json!([1, 2, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should return null if not found
  assert_eq!(result.unwrap(), json!(null));
}

#[tokio::test]
async fn test_array_index_of_empty_array() {
  let mut node = ArrayIndexOf::new(json!(1), ArrayIndexOfMode::First);
  let input = json!([]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!(null));
}

#[tokio::test]
async fn test_array_index_of_non_array_input() {
  let mut node = ArrayIndexOf::new(json!(1), ArrayIndexOfMode::First);
  let input = json!(42);
  let input_stream = Box::pin(stream::iter(vec![input.clone()]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;
  // Non-array input should return null
  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!(null));
}

#[tokio::test]
async fn test_array_index_of_stream() {
  let mut node = ArrayIndexOf::new(json!(2), ArrayIndexOfMode::First);
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

  assert_eq!(results.len(), 3);
  assert_eq!(results[0], json!(1));
  assert_eq!(results[1], json!(null));
  assert_eq!(results[2], json!(0));
}

#[tokio::test]
async fn test_array_index_of_empty_stream() {
  let mut node = ArrayIndexOf::new(json!(1), ArrayIndexOfMode::First);
  let input_stream = Box::pin(stream::empty::<serde_json::Value>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_array_index_of_config_access() {
  let mut node = ArrayIndexOf::new(json!(1), ArrayIndexOfMode::First).with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}
