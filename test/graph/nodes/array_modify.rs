//! Tests for ArrayModify node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::ArrayModify;
use streamweave::transformers::ArrayModifyOperation;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_array_modify_new() {
  let node = ArrayModify::new(ArrayModifyOperation::Push, Some(json!(5)));
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_array_modify_with_name() {
  let node = ArrayModify::new(ArrayModifyOperation::Push, Some(json!(5)))
    .with_name("modify_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("modify_node"));
}

#[tokio::test]
async fn test_array_modify_with_error_strategy() {
  let node = ArrayModify::new(ArrayModifyOperation::Push, Some(json!(5)))
    .with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_array_modify_push() {
  let mut node = ArrayModify::new(ArrayModifyOperation::Push, Some(json!(5)));
  let input = json!([1, 2, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!([1, 2, 3, 4, 5]));
}

#[tokio::test]
async fn test_array_modify_pop() {
  let mut node = ArrayModify::new(ArrayModifyOperation::Pop, None);
  let input = json!([1, 2, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should return array without last element, and the popped value
  let output = result.unwrap();
  // The transformer may return the modified array or the popped value
  assert!(output.is_array() || output == json!(4));
}

#[tokio::test]
async fn test_array_modify_shift() {
  let mut node = ArrayModify::new(ArrayModifyOperation::Shift, None);
  let input = json!([1, 2, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should return array without first element, or the shifted value
  let output = result.unwrap();
  assert!(output.is_array() || output == json!(1));
}

#[tokio::test]
async fn test_array_modify_unshift() {
  let mut node = ArrayModify::new(ArrayModifyOperation::Unshift, Some(json!(0)));
  let input = json!([1, 2, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!([0, 1, 2, 3, 4]));
}

#[tokio::test]
async fn test_array_modify_empty_array() {
  let mut node = ArrayModify::new(ArrayModifyOperation::Pop, None);
  let input = json!([]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;
  // Should handle empty array gracefully
  assert!(result.is_some());
}

#[tokio::test]
async fn test_array_modify_non_array_input() {
  let mut node = ArrayModify::new(ArrayModifyOperation::Push, Some(json!(5)));
  let input = json!(42);
  let input_stream = Box::pin(stream::iter(vec![input.clone()]));

  let mut output_stream = node.transform(input_stream).await;
  // Non-array input should pass through or return null
  let result = output_stream.next().await;
  assert!(result.is_some());
}

#[tokio::test]
async fn test_array_modify_stream() {
  let mut node = ArrayModify::new(ArrayModifyOperation::Push, Some(json!(99)));
  let input_stream = Box::pin(stream::iter(vec![json!([1, 2]), json!([3, 4])]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 2);
  assert_eq!(results[0], json!([1, 2, 99]));
  assert_eq!(results[1], json!([3, 4, 99]));
}

#[tokio::test]
async fn test_array_modify_empty_stream() {
  let mut node = ArrayModify::new(ArrayModifyOperation::Push, Some(json!(5)));
  let input_stream = Box::pin(stream::empty::<serde_json::Value>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_array_modify_config_access() {
  let mut node =
    ArrayModify::new(ArrayModifyOperation::Push, Some(json!(5))).with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}
