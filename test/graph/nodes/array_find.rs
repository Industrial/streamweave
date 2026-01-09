//! Tests for ArrayFind node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::ArrayFind;
use streamweave::transformers::FindOperation;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_array_find_new() {
  let node = ArrayFind::new(FindOperation::Find, json!(5));
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_array_find_with_name() {
  let node = ArrayFind::new(FindOperation::Find, json!(5)).with_name("find_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("find_node"));
}

#[tokio::test]
async fn test_array_find_with_error_strategy() {
  let node = ArrayFind::new(FindOperation::Find, json!(5)).with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_array_find_operation_find() {
  let mut node = ArrayFind::new(FindOperation::Find, json!(3));
  let input = json!([1, 2, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should return the found element
  assert_eq!(result.unwrap(), json!(3));
}

#[tokio::test]
async fn test_array_find_operation_find_index() {
  let mut node = ArrayFind::new(FindOperation::FindIndex, json!(3));
  let input = json!([1, 2, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should return the index
  assert_eq!(result.unwrap(), json!(2));
}

#[tokio::test]
async fn test_array_find_not_found() {
  let mut node = ArrayFind::new(FindOperation::Find, json!(99));
  let input = json!([1, 2, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should return null if not found
  assert_eq!(result.unwrap(), json!(null));
}

#[tokio::test]
async fn test_array_find_empty_array() {
  let mut node = ArrayFind::new(FindOperation::Find, json!(1));
  let input = json!([]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!(null));
}

#[tokio::test]
async fn test_array_find_non_array_input() {
  let mut node = ArrayFind::new(FindOperation::Find, json!(1));
  let input = json!(42);
  let input_stream = Box::pin(stream::iter(vec![input.clone()]));

  let mut output_stream = node.transform(input_stream).await;
  // Non-array input should pass through or return null
  let result = output_stream.next().await;
  assert!(result.is_some());
}

#[tokio::test]
async fn test_array_find_stream() {
  let mut node = ArrayFind::new(FindOperation::Find, json!(2));
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
  assert_eq!(results[0], json!(2));
  assert_eq!(results[1], json!(null));
  assert_eq!(results[2], json!(2));
}

#[tokio::test]
async fn test_array_find_empty_stream() {
  let mut node = ArrayFind::new(FindOperation::Find, json!(1));
  let input_stream = Box::pin(stream::empty::<serde_json::Value>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_array_find_config_access() {
  let mut node = ArrayFind::new(FindOperation::Find, json!(1)).with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}
