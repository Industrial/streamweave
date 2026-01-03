//! Key-based router tests
//!
//! This module provides integration tests for KeyBasedRouter functionality.

use futures::{StreamExt, stream};
use std::pin::Pin;
use streamweave::graph::KeyBasedRouter;

#[tokio::test]
async fn test_key_based_router_basic() {
  let mut router = KeyBasedRouter::new(
    |x: &i32| *x % 3, // Key function: modulo 3
    vec![0, 1, 2],
  );
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3, 4, 5, 6]));

  let mut output_streams = router.route_stream(input_stream).await;

  assert_eq!(output_streams.len(), 3);

  // Collect from each stream
  let mut results = Vec::new();
  for (_, mut stream) in output_streams {
    let mut stream_results = Vec::new();
    while let Some(item) = stream.next().await {
      stream_results.push(item);
    }
    results.push(stream_results);
  }

  // Items should be distributed by key (mod 3)
  // All streams should receive some items
  assert!(results.iter().any(|r| !r.is_empty()));

  // Verify all items were distributed
  let total_items: usize = results.iter().map(|r| r.len()).sum();
  assert_eq!(total_items, 6);
}

#[test]
fn test_key_based_router_output_ports() {
  let router = KeyBasedRouter::<i32, i32>::new(|x| *x, vec![0, 1, 2]);
  assert_eq!(router.output_ports(), vec![0, 1, 2]);
}

#[tokio::test]
async fn test_key_based_router_with_mapping() {
  use std::collections::HashMap;

  let mut key_to_port = HashMap::new();
  key_to_port.insert(0, 0);
  key_to_port.insert(1, 1);
  key_to_port.insert(2, 2);

  let mut router = KeyBasedRouter::with_mapping(|x: &i32| *x % 3, key_to_port, vec![0, 1, 2]);

  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![3, 4, 5])); // Keys: 0, 1, 2

  let mut output_streams = router.route_stream(input_stream).await;
  assert_eq!(output_streams.len(), 3);

  // Collect from each stream
  let mut results = Vec::new();
  for (_, mut stream) in output_streams {
    let mut stream_results = Vec::new();
    while let Some(item) = stream.next().await {
      stream_results.push(item);
    }
    results.push(stream_results);
  }

  // Verify items were distributed
  let total_items: usize = results.iter().map(|r| r.len()).sum();
  assert_eq!(total_items, 3);
}

#[tokio::test]
async fn test_key_based_router_single_port() {
  let mut router = KeyBasedRouter::new(|x: &i32| *x, vec![0]);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_streams = router.route_stream(input_stream).await;
  assert_eq!(output_streams.len(), 1);

  let mut results = Vec::new();
  let stream = &mut output_streams[0].1;
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  // All items go to the single port
  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_key_based_router_empty_stream() {
  let mut router = KeyBasedRouter::new(|x: &i32| *x % 2, vec![0, 1]);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![]));

  let output_streams = router.route_stream(input_stream).await;
  assert_eq!(output_streams.len(), 2);

  // Both streams should be empty
  let mut stream1 = &mut output_streams[0].1;
  let mut stream2 = &mut output_streams[1].1;

  assert!(stream1.next().await.is_none());
  assert!(stream2.next().await.is_none());
}

// Tests moved from src/
use futures::stream;

#[tokio::test]
async fn test_key_based_router() {
  let mut router = KeyBasedRouter::new(
    |x: &i32| *x % 3, // Key function: modulo 3
    vec![0, 1, 2],
  );
  let input_stream: Pin<Box<dyn Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3, 4, 5, 6]));

  let output_streams = router.route_stream(input_stream).await;

  assert_eq!(output_streams.len(), 3);

  // Collect from each stream
  use futures::StreamExt;
  let mut results = Vec::new();
  for (_, mut stream) in output_streams {
    let mut stream_results = Vec::new();
    while let Some(item) = stream.next().await {
      stream_results.push(item);
    }
    results.push(stream_results);
  }

  // Items should be distributed by key (mod 3)
  // All streams should receive some items
  assert!(results.iter().any(|r| !r.is_empty()));
}

#[test]
fn test_key_based_output_ports() {
  let router = KeyBasedRouter::<i32, i32>::new(|x| *x, vec![0, 1, 2]);
  assert_eq!(
    router.output_port_names(),
    vec!["out".to_string(), "out_1".to_string(), "out_2".to_string()]
  );
}
