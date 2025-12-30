//! Round-robin router tests
//!
//! This module provides integration tests for RoundRobinRouter functionality.

use futures::{StreamExt, stream};
use std::pin::Pin;
use streamweave_graph::RoundRobinRouter;

#[tokio::test]
async fn test_round_robin_router_basic() {
  let mut router = RoundRobinRouter::new(vec![0, 1, 2]);
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

  // Items should be distributed round-robin
  // Stream 0: [1, 4]
  // Stream 1: [2, 5]
  // Stream 2: [3, 6]
  assert_eq!(results[0], vec![1, 4]);
  assert_eq!(results[1], vec![2, 5]);
  assert_eq!(results[2], vec![3, 6]);
}

#[test]
fn test_round_robin_router_output_ports() {
  let router = RoundRobinRouter::<i32>::new(vec![0, 1, 2]);
  assert_eq!(router.output_ports(), vec![0, 1, 2]);
}

#[tokio::test]
async fn test_round_robin_router_single_port() {
  let mut router = RoundRobinRouter::new(vec![0]);
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
async fn test_round_robin_router_empty_stream() {
  let mut router = RoundRobinRouter::new(vec![0, 1]);
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

#[tokio::test]
async fn test_round_robin_router_uneven_distribution() {
  let mut router = RoundRobinRouter::new(vec![0, 1]);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3, 4, 5]));

  let mut output_streams = router.route_stream(input_stream).await;
  assert_eq!(output_streams.len(), 2);

  let mut results = Vec::new();
  for (_, mut stream) in output_streams {
    let mut stream_results = Vec::new();
    while let Some(item) = stream.next().await {
      stream_results.push(item);
    }
    results.push(stream_results);
  }

  // Items should be distributed round-robin
  // Stream 0: [1, 3, 5]
  // Stream 1: [2, 4]
  assert_eq!(results[0], vec![1, 3, 5]);
  assert_eq!(results[1], vec![2, 4]);
}
