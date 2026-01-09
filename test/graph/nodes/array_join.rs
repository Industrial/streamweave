//! Tests for ArrayJoin node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::ArrayJoin;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_array_join_new() {
  let node = ArrayJoin::new(",");
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_array_join_with_name() {
  let node = ArrayJoin::new(",").with_name("join_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("join_node"));
}

#[tokio::test]
async fn test_array_join_with_error_strategy() {
  let node = ArrayJoin::new(",").with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_array_join_basic() {
  let mut node = ArrayJoin::new(",");
  let input = json!([1, 2, 3, 4]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should join array elements with delimiter
  assert_eq!(result.unwrap(), json!("1,2,3,4"));
}

#[tokio::test]
async fn test_array_join_custom_delimiter() {
  let mut node = ArrayJoin::new(" - ");
  let input = json!(["a", "b", "c"]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!("a - b - c"));
}

#[tokio::test]
async fn test_array_join_empty_array() {
  let mut node = ArrayJoin::new(",");
  let input = json!([]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Empty array should produce empty string
  assert_eq!(result.unwrap(), json!(""));
}

#[tokio::test]
async fn test_array_join_single_element() {
  let mut node = ArrayJoin::new(",");
  let input = json!([42]);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!("42"));
}

#[tokio::test]
async fn test_array_join_non_array_input() {
  let mut node = ArrayJoin::new(",");
  let input = json!(42);
  let input_stream = Box::pin(stream::iter(vec![input.clone()]));

  let mut output_stream = node.transform(input_stream).await;
  // Non-array input should pass through or be converted
  let result = output_stream.next().await;
  assert!(result.is_some());
}

#[tokio::test]
async fn test_array_join_stream() {
  let mut node = ArrayJoin::new("|");
  let input_stream = Box::pin(stream::iter(vec![
    json!([1, 2]),
    json!([3, 4]),
    json!([5, 6]),
  ]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 3);
  assert_eq!(results[0], json!("1|2"));
  assert_eq!(results[1], json!("3|4"));
  assert_eq!(results[2], json!("5|6"));
}

#[tokio::test]
async fn test_array_join_empty_stream() {
  let mut node = ArrayJoin::new(",");
  let input_stream = Box::pin(stream::empty::<serde_json::Value>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_array_join_config_access() {
  let mut node = ArrayJoin::new(",").with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}
