//! Tests for Repeat node

use futures::StreamExt;
use futures::stream;
use streamweave::error::ErrorStrategy;
use streamweave::graph::nodes::Repeat;
use streamweave::{Transformer, TransformerConfig};

#[tokio::test]
async fn test_repeat_new() {
  let node = Repeat::<i32>::new(3);
  assert!(node.get_config_impl().name().is_none());
}

#[tokio::test]
async fn test_repeat_with_name() {
  let node = Repeat::<i32>::new(3).with_name("repeat_node".to_string());
  assert_eq!(node.get_config_impl().name(), Some("repeat_node"));
}

#[tokio::test]
async fn test_repeat_with_error_strategy() {
  let node = Repeat::<i32>::new(3).with_error_strategy(ErrorStrategy::Skip);
  assert!(matches!(
    node.get_config_impl().error_strategy(),
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_repeat_basic() {
  let mut node = Repeat::new(3);
  let input_stream = Box::pin(stream::iter(vec![1, 2]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  // Should repeat each item 3 times
  assert_eq!(results, vec![1, 1, 1, 2, 2, 2]);
}

#[tokio::test]
async fn test_repeat_single_item() {
  let mut node = Repeat::new(5);
  let input_stream = Box::pin(stream::iter(vec![42]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 5);
  assert!(results.iter().all(|&x| x == 42));
}

#[tokio::test]
async fn test_repeat_empty() {
  let mut node = Repeat::new(3);
  let input_stream = Box::pin(stream::iter(vec![] as Vec<i32>));

  let mut output_stream = node.transform(input_stream).await;
  assert!(output_stream.next().await.is_none());
}

#[tokio::test]
async fn test_repeat_strings() {
  let mut node = Repeat::new(2);
  let input_stream = Box::pin(stream::iter(vec!["hello".to_string(), "world".to_string()]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 4);
  assert_eq!(results[0], "hello");
  assert_eq!(results[1], "hello");
  assert_eq!(results[2], "world");
  assert_eq!(results[3], "world");
}

#[tokio::test]
async fn test_repeat_large_count() {
  let mut node = Repeat::new(100);
  let input_stream = Box::pin(stream::iter(vec![1]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 100);
  assert!(results.iter().all(|&x| x == 1));
}

#[tokio::test]
async fn test_repeat_stream() {
  let mut node = Repeat::new(2);
  let input_stream = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_stream = node.transform(input_stream).await;
  let mut results = Vec::new();
  while let Some(item) = output_stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 1, 2, 2, 3, 3]);
}

#[tokio::test]
async fn test_repeat_config_access() {
  let mut node = Repeat::<i32>::new(3).with_name("test".to_string());
  let config = node.get_config_impl();
  assert_eq!(config.name(), Some("test"));

  let mut config_mut = node.get_config_mut_impl();
  config_mut.name = Some("updated".to_string());
  assert_eq!(node.get_config_impl().name(), Some("updated"));
}

#[tokio::test]
async fn test_repeat_clone() {
  let node1 = Repeat::<i32>::new(3).with_name("original".to_string());
  let node2 = node1.clone();

  assert_eq!(
    node1.get_config_impl().name(),
    node2.get_config_impl().name()
  );
}
