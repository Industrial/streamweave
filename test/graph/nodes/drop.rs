//! Tests for Drop node

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::Drop;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_drop_new() {
  let node = Drop::new(|x: &i32| *x > 5);
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_drop_with_name() {
  let node = Drop::new(|x: &i32| *x > 5).with_name("drop_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("drop_node"));
}

#[tokio::test]
async fn test_drop_with_error_strategy() {
  let node = Drop::new(|x: &i32| *x > 5).with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_drop_basic() {
  let mut node = Drop::new(|x: &i32| *x > 3);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should drop items > 3, keeping 1, 2, 3
  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_drop_all() {
  let mut node = Drop::new(|_x: &i32| true);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = node.transform(input_stream).await;
  // Should drop all items
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_drop_none() {
  let mut node = Drop::new(|_x: &i32| false);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should keep all items
  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_drop_empty() {
  let mut node = Drop::new(|x: &i32| *x > 2);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_drop_strings() {
  let mut node = Drop::new(|x: &String| x.len() > 3);
  let input_stream = Box::pin(stream::iter(vec![
    "a".to_string(),
    "ab".to_string(),
    "abc".to_string(),
    "abcd".to_string(),
  ]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should drop strings with length > 3
  assert_eq!(results.len(), 3);
  assert_eq!(results[0], "a");
  assert_eq!(results[1], "ab");
  assert_eq!(results[2], "abc");
}

#[tokio::test]
async fn test_drop_stream() {
  let mut node = Drop::new(|x: &i32| *x % 2 == 0);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3, 4, 5, 6]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should drop even numbers, keeping odd
  assert_eq!(results, vec![1, 3, 5]);
}

#[tokio::test]
async fn test_drop_config_access() {
  let mut node = Drop::new(|x: &i32| *x > 5).with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_drop_clone() {
  let node1 = Drop::new(|x: &i32| *x > 5).with_name("original".to_string());
  let node2 = node1.clone();

  assert_eq!(
    node1.get_config_impl().name(),
    node2.get_config_impl().name()
  );
}
