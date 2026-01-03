//! Broadcast router tests
//!
//! This module provides integration tests for BroadcastRouter functionality.

use futures::{StreamExt, stream};
use std::pin::Pin;
use streamweave::graph::BroadcastRouter;

#[tokio::test]
async fn test_broadcast_router_basic() {
  let mut router = BroadcastRouter::new(vec![0, 1, 2]);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));

  let output_streams = router.route_stream(input_stream).await;

  assert_eq!(output_streams.len(), 3);

  // Collect from first stream
  let mut results0 = Vec::new();
  let mut stream0 = &mut output_streams[0].1;
  while let Some(item) = stream0.next().await {
    results0.push(item);
    if results0.len() >= 3 {
      break;
    }
  }

  assert_eq!(results0, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_broadcast_router_all_streams() {
  let mut router = BroadcastRouter::new(vec![0, 1]);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_streams = router.route_stream(input_stream).await;

  assert_eq!(output_streams.len(), 2);

  // Collect from both streams
  let mut results1 = Vec::new();
  let mut results2 = Vec::new();

  let stream1 = &mut output_streams[0].1;
  let stream2 = &mut output_streams[1].1;

  // Collect from stream1
  while let Some(item) = stream1.next().await {
    results1.push(item);
    if results1.len() >= 3 {
      break;
    }
  }

  // Collect from stream2
  while let Some(item) = stream2.next().await {
    results2.push(item);
    if results2.len() >= 3 {
      break;
    }
  }

  // Both should receive all items (broadcast)
  assert_eq!(results1, vec![1, 2, 3]);
  assert_eq!(results2, vec![1, 2, 3]);
}

#[test]
fn test_broadcast_router_output_ports() {
  let router = BroadcastRouter::<i32>::new(vec![0, 1, 2, 3]);
  assert_eq!(router.output_ports(), vec![0, 1, 2, 3]);
}

#[test]
fn test_broadcast_router_empty_ports() {
  let router = BroadcastRouter::<i32>::new(vec![]);
  assert_eq!(router.output_ports(), vec![]);
}

#[tokio::test]
async fn test_broadcast_router_empty_stream() {
  let mut router = BroadcastRouter::new(vec![0, 1]);
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
async fn test_broadcast_router_single_port() {
  let mut router = BroadcastRouter::new(vec![0]);
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_streams = router.route_stream(input_stream).await;
  assert_eq!(output_streams.len(), 1);

  let mut results = Vec::new();
  let stream = &mut output_streams[0].1;
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

// Tests moved from src/
use futures::stream;

#[tokio::test]
async fn test_broadcast_router() {
  let mut router = BroadcastRouter::new(vec![0, 1, 2]);
  let input_stream: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(vec![1, 2, 3]));

  let mut output_streams = router.route_stream(input_stream).await;

  assert_eq!(output_streams.len(), 3);

  // Collect from first stream
  use futures::StreamExt;
  let stream0 = &mut output_streams[0].1;
  let mut results0 = Vec::new();
  while let Some(item) = stream0.next().await {
    results0.push(item);
    if results0.len() >= 3 {
      break;
    }
  }

  assert_eq!(results0, vec![1, 2, 3]);
}

#[test]
fn test_broadcast_output_ports() {
  let router = BroadcastRouter::<i32>::new(vec![0, 1, 2]);
  assert_eq!(
    router.output_port_names(),
    vec!["out".to_string(), "out_1".to_string(), "out_2".to_string()]
  );
}
