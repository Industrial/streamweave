//! # Merge Router
//!
//! This module provides a MergeRouter that merges multiple input streams
//! into a single output stream according to a merge strategy.

use crate::graph::router::InputRouter;
use crate::transformers::ordered_merge::ordered_merge_transformer::MergeStrategy;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use futures::stream::select_all;
use std::marker::PhantomData;
use std::pin::Pin;

/// A router that merges multiple input streams into a single output stream.
///
/// This router implements fan-in patterns, combining items from multiple input
/// ports into a single stream according to a merge strategy.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::routers::MergeRouter;
/// use streamweave::transformers::ordered_merge::ordered_merge_transformer::MergeStrategy;
/// use futures::stream;
/// use std::pin::Pin;
///
/// let mut router = MergeRouter::new(
///     vec![0, 1, 2],              // Input ports
///     MergeStrategy::Interleave,  // Merge strategy
/// );
/// let streams = vec![
///     (0, Box::pin(stream::iter(vec![1, 4])) as Pin<Box<dyn Stream<Item = i32> + Send>>),
///     (1, Box::pin(stream::iter(vec![2, 5])) as Pin<Box<dyn Stream<Item = i32> + Send>>),
///     (2, Box::pin(stream::iter(vec![3, 6])) as Pin<Box<dyn Stream<Item = i32> + Send>>),
/// ];
///
/// let merged = router.route_streams(streams).await;
/// // Merged stream contains items interleaved from all inputs
/// ```
pub struct MergeRouter<I> {
  /// The input port indices this router expects
  expected_ports: Vec<usize>,
  /// The merge strategy to use
  strategy: MergeStrategy,
  /// Phantom data to consume the type parameter
  _phantom: PhantomData<I>,
}

impl<I> MergeRouter<I> {
  /// Creates a new MergeRouter with the specified input ports and strategy.
  ///
  /// # Arguments
  ///
  /// * `expected_ports` - Vector of port indices to merge from
  /// * `strategy` - The merge strategy to use
  ///
  /// # Returns
  ///
  /// A new `MergeRouter` instance.
  pub fn new(expected_ports: Vec<usize>, strategy: MergeStrategy) -> Self {
    Self {
      expected_ports,
      strategy,
      _phantom: PhantomData,
    }
  }
}

#[async_trait]
impl<I> InputRouter<I> for MergeRouter<I>
where
  I: Send + Sync + 'static,
{
  async fn route_streams(
    &mut self,
    streams: Vec<(usize, Pin<Box<dyn Stream<Item = I> + Send>>)>,
  ) -> Pin<Box<dyn Stream<Item = I> + Send>> {
    if streams.is_empty() {
      return Box::pin(futures::stream::empty());
    }

    match self.strategy {
      MergeStrategy::Interleave => {
        // Fair interleaving using select_all
        let stream_vec: Vec<_> = streams.into_iter().map(|(_, stream)| stream).collect();
        Box::pin(select_all(stream_vec))
      }
      MergeStrategy::Sequential => {
        // Process streams in order, exhaust each before moving to next
        Box::pin(async_stream::stream! {
          for (_, mut stream) in streams {
            while let Some(item) = stream.next().await {
              yield item;
            }
          }
        })
      }
      MergeStrategy::RoundRobin => {
        // Round-robin: take one from each stream in turn
        Box::pin(async_stream::stream! {
          let mut streams: Vec<_> = streams.into_iter().map(|(_, s)| s).collect();
          let mut indices: Vec<usize> = (0..streams.len()).collect();

          loop {
            let mut any_ready = false;
            let mut items = Vec::new();

            // Try to get one item from each stream in round-robin order
            for &idx in &indices {
              if let Some(item) = streams[idx].next().await {
                items.push((idx, item));
                any_ready = true;
              }
            }

            if !any_ready {
              break;
            }

            // Yield items in round-robin order
            items.sort_by_key(|(idx, _)| *idx);
            for (_, item) in items {
              yield item;
            }

            // Remove exhausted streams
            indices.retain(|&_idx| {
              // Check if stream is exhausted (simplified - in practice would need peek)
              true // Keep all for now, streams will naturally end
            });
          }
        })
      }
      MergeStrategy::Priority => {
        // Priority: process streams in order of priority (lower index = higher priority)
        Box::pin(async_stream::stream! {
          let streams: Vec<_> = streams.into_iter().map(|(_, s)| s).collect();

          // Process streams in priority order (index order)
          for mut stream in streams {
            while let Some(item) = stream.next().await {
              yield item;
            }
          }
        })
      }
    }
  }

  fn expected_ports(&self) -> Vec<usize> {
    self.expected_ports.clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::transformers::ordered_merge::ordered_merge_transformer::MergeStrategy;
  use futures::stream;

  #[tokio::test]
  async fn test_merge_router_interleave() {
    let mut router = MergeRouter::new(vec![0, 1], MergeStrategy::Interleave);
    let stream1: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(vec![1, 3, 5]));
    let stream2: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(vec![2, 4, 6]));

    let streams = vec![(0, stream1), (1, stream2)];
    let mut merged = router.route_streams(streams).await;

    // Collect results
    use futures::StreamExt;
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
    let stream1: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(vec![1, 2, 3]));
    let stream2: Pin<Box<dyn Stream<Item = i32> + Send>> = Box::pin(stream::iter(vec![4, 5, 6]));

    let streams = vec![(0, stream1), (1, stream2)];
    let mut merged = router.route_streams(streams).await;

    // Collect results
    use futures::StreamExt;
    let mut results = Vec::new();
    while let Some(item) = merged.next().await {
      results.push(item);
    }

    // Sequential: all from stream1, then all from stream2
    assert_eq!(results, vec![1, 2, 3, 4, 5, 6]);
  }

  #[test]
  fn test_merge_expected_ports() {
    let router = MergeRouter::<i32>::new(vec![0, 1, 2], MergeStrategy::Interleave);
    assert_eq!(router.expected_ports(), vec![0, 1, 2]);
  }
}
