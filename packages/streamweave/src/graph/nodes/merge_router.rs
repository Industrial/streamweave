//! # Merge Router
//!
//! This module provides a MergeRouter that merges multiple input streams
//! into a single output stream according to a merge strategy.

use crate::router::InputRouter;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use futures::stream::select_all;
use std::marker::PhantomData;
use std::pin::Pin;

/// Defines the ordering strategy for merging multiple streams.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MergeStrategy {
  /// Process streams in order, exhaust one before moving to next.
  /// Elements from stream 0 come first, then stream 1, etc.
  Sequential,

  /// Take one element from each stream in turn (round-robin).
  /// Stream 0, Stream 1, Stream 2, Stream 0, Stream 1, ...
  RoundRobin,

  /// Process streams based on priority index (lower index = higher priority).
  /// When higher priority stream has elements, they are processed first.
  Priority,

  /// Fair interleaving using select_all (default futures behavior).
  /// Whichever stream has an element ready gets processed.
  #[default]
  Interleave,
}

/// A router that merges multiple input streams into a single output stream.
///
/// This router implements fan-in patterns, combining items from multiple input
/// ports into a single stream according to a merge strategy.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::routers::MergeRouter;
/// use streamweave::graph::routers::MergeStrategy;
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
  /// The number of input ports this router expects (used internally for port name generation)
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
  /// * `expected_ports` - Vector of port counts (used internally to determine number of ports).
  ///   Port names are generated automatically as "in", "in_1", "in_2", etc.
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
    streams: Vec<(String, Pin<Box<dyn Stream<Item = I> + Send>>)>,
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

  fn expected_port_names(&self) -> Vec<String> {
    // Generate port names based on number of ports
    (0..self.expected_ports.len())
      .map(|i| {
        if i == 0 {
          "in".to_string()
        } else {
          format!("in_{}", i)
        }
      })
      .collect()
  }
}
