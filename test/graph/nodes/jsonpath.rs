//! Tests for JsonPath node

use futures::StreamExt;
use futures::stream;
use serde_json::json;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::JsonPath;
use streamweave::transformers::JsonPathOperation;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_jsonpath_new() {
  let node = JsonPath::new("$.name", JsonPathOperation::Get, None);
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_jsonpath_with_name() {
  let node =
    JsonPath::new("$.name", JsonPathOperation::Get, None).with_name("jsonpath_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("jsonpath_node"));
}

#[tokio::test]
async fn test_jsonpath_with_error_strategy() {
  let node =
    JsonPath::new("$.name", JsonPathOperation::Get, None).with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_jsonpath_get_operation() {
  let mut node = JsonPath::new("$.name", JsonPathOperation::Get, None);
  let input = json!({"name": "Alice", "age": 30});
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should extract the value at the JSONPath
  let output = result.unwrap();
  assert_eq!(output, json!("Alice"));
}

#[tokio::test]
async fn test_jsonpath_get_nested() {
  let mut node = JsonPath::new("$.user.name", JsonPathOperation::Get, None);
  let input = json!({"user": {"name": "Bob", "age": 25}});
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  let output = result.unwrap();
  assert_eq!(output, json!("Bob"));
}

#[tokio::test]
async fn test_jsonpath_get_array_index() {
  let mut node = JsonPath::new("$.items[0]", JsonPathOperation::Get, None);
  let input = json!({"items": [1, 2, 3]});
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  assert_eq!(result.unwrap(), json!(1));
}

#[tokio::test]
async fn test_jsonpath_compare_operation() {
  let mut node = JsonPath::new("$.age", JsonPathOperation::Compare, Some(json!(30)));
  let input = json!({"name": "Alice", "age": 30});
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should return boolean result of comparison
  let output = result.unwrap();
  assert!(output.is_boolean());
}

#[tokio::test]
async fn test_jsonpath_path_not_found() {
  let mut node = JsonPath::new("$.nonexistent", JsonPathOperation::Get, None);
  let input = json!({"name": "Alice"});
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;

  assert!(result.is_some());
  // Should return null if path not found
  assert_eq!(result.unwrap(), json!(null));
}

#[tokio::test]
async fn test_jsonpath_non_object_input() {
  let mut node = JsonPath::new("$.name", JsonPathOperation::Get, None);
  let input = json!(42);
  let input_stream = Box::pin(stream::iter(vec![input]));

  let mut output_stream = node.transform(input_stream).await;
  let result = output_stream.next().await;
  // Non-object input should return null or error
  assert!(result.is_some());
}

#[tokio::test]
async fn test_jsonpath_stream() {
  let mut node = JsonPath::new("$.value", JsonPathOperation::Get, None);
  let input_stream = Box::pin(stream::iter(vec![
    json!({"value": 1}),
    json!({"value": 2}),
    json!({"value": 3}),
  ]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 3);
  assert_eq!(results[0], json!(1));
  assert_eq!(results[1], json!(2));
  assert_eq!(results[2], json!(3));
}

#[tokio::test]
async fn test_jsonpath_empty_stream() {
  let mut node = JsonPath::new("$.name", JsonPathOperation::Get, None);
  let input_stream = Box::pin(stream::empty::<serde_json::Value>());

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_jsonpath_config_access() {
  let mut node =
    JsonPath::new("$.name", JsonPathOperation::Get, None).with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_jsonpath_clone() {
  let node1 =
    JsonPath::new("$.name", JsonPathOperation::Get, None).with_name("original".to_string());
  let node2 = node1.clone();

  assert_eq!(
    node1.get_config_impl().name(),
    node2.get_config_impl().name()
  );
}
