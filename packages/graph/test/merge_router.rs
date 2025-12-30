//! Merge router tests
//!
//! This module provides integration tests for MergeRouter functionality.

use futures::{StreamExt, stream};
use std::pin::Pin;
use streamweave_graph::MergeRouter;
use streamweave_transformers::MergeStrategy;

#[tokio::test]
async fn test_merge_router_interleave() {
  let mut router = MergeRouter::new(vec![0, 1], MergeStrategy::Interleave);
  let stream1: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 3, 5]));
  let stream2: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![2, 4, 6]));

  let streams = vec![(0, stream1), (1, stream2)];
  let mut merged = router.route_streams(streams).await;

  // Collect results
  let mut results = Vec::new();
  while let Some(item) = merged.next().await {
    results.push(item);
  }

  // Should have all items (order may vary due to interleaving)
  assert_eq!(results.len(), 6);
  assert!(results.contains(&1));
  assert!(results.contains(&2));
  assert!(results.contains(&3));
  assert!(results.contains(&4));
  assert!(results.contains(&5));
  assert!(results.contains(&6));
}

#[tokio::test]
async fn test_merge_router_sequential() {
  let mut router = MergeRouter::new(vec![0, 1], MergeStrategy::Sequential);
  let stream1: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));
  let stream2: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![4, 5, 6]));

  let streams = vec![(0, stream1), (1, stream2)];
  let mut merged = router.route_streams(streams).await;

  // Collect results
  let mut results = Vec::new();
  while let Some(item) = merged.next().await {
    results.push(item);
  }

  // Sequential: all from stream1, then all from stream2
  assert_eq!(results, vec![1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_merge_router_expected_ports() {
  let router = MergeRouter::<i32>::new(vec![0, 1, 2], MergeStrategy::Interleave);
  assert_eq!(router.expected_ports(), vec![0, 1, 2]);
}

#[tokio::test]
async fn test_merge_router_empty_streams() {
  let mut router = MergeRouter::new(vec![0, 1], MergeStrategy::Interleave);
  let streams: Vec<(usize, Pin<Box<dyn futures::Stream<Item = i32> + Send>>)> = vec![];

  let mut merged = router.route_streams(streams).await;
  assert!(merged.next().await.is_none());
}

#[tokio::test]
async fn test_merge_router_single_stream() {
  let mut router = MergeRouter::new(vec![0], MergeStrategy::Interleave);
  let stream1: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));

  let streams = vec![(0, stream1)];
  let mut merged = router.route_streams(streams).await;

  let mut results = Vec::new();
  while let Some(item) = merged.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![1, 2, 3]);
}

#[tokio::test]
async fn test_merge_router_round_robin() {
  let mut router = MergeRouter::new(vec![0, 1], MergeStrategy::RoundRobin);
  let stream1: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 3, 5]));
  let stream2: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![2, 4, 6]));

  let streams = vec![(0, stream1), (1, stream2)];
  let mut merged = router.route_streams(streams).await;

  let mut results = Vec::new();
  while let Some(item) = merged.next().await {
    results.push(item);
  }

  // Should have all items
  assert_eq!(results.len(), 6);
  assert!(results.contains(&1));
  assert!(results.contains(&2));
  assert!(results.contains(&3));
  assert!(results.contains(&4));
  assert!(results.contains(&5));
  assert!(results.contains(&6));
}

#[tokio::test]
async fn test_merge_router_priority() {
  let mut router = MergeRouter::new(vec![0, 1], MergeStrategy::Priority);
  let stream1: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));
  let stream2: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![4, 5, 6]));

  let streams = vec![(0, stream1), (1, stream2)];
  let mut merged = router.route_streams(streams).await;

  let mut results = Vec::new();
  while let Some(item) = merged.next().await {
    results.push(item);
  }

  // Priority: all from stream1 (lower index = higher priority), then all from stream2
  assert_eq!(results, vec![1, 2, 3, 4, 5, 6]);
}
