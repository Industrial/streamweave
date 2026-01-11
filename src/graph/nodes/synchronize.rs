//! Synchronize router for synchronizing multiple input streams.
//!
//! This module provides [`Synchronize`], an input router that synchronizes multiple
//! input streams, waiting for items from all input ports before proceeding. It
//! implements [`InputRouter`] for use in StreamWeave graphs, enabling
//! synchronization patterns where all inputs must be ready before processing.
//!
//! # Overview
//!
//! [`Synchronize`] is useful for synchronizing multiple input streams in
//! graph-based pipelines. It collects items from all input ports and emits them
//! together when all ports have items available, making it ideal for
//! synchronization patterns and barrier synchronization.
//!
//! # Key Concepts
//!
//! - **Stream Synchronization**: Waits for items from all input ports before proceeding
//! - **Barrier Pattern**: Implements barrier synchronization across multiple streams
//! - **Input Router**: Implements `InputRouter` for graph integration
//! - **Configurable Inputs**: Supports configurable number of expected inputs
//!
//! # Core Types
//!
//! - **[`Synchronize<T>`]**: Router that synchronizes multiple input streams
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Synchronize;
//!
//! // Create a synchronize router for 3 inputs
//! let sync = Synchronize::<i32>::new(3);
//! ```
//!
//! ## With Different Types
//!
//! ```rust
//! use streamweave::graph::nodes::Synchronize;
//!
//! // Synchronize string streams
//! let sync = Synchronize::<String>::new(2);
//! ```
//!
//! # Design Decisions
//!
//! - **Barrier Synchronization**: Waits for all inputs before proceeding for
//!   predictable synchronization
//! - **Input Router Trait**: Implements `InputRouter` for integration with
//!   graph system
//! - **Configurable Inputs**: Supports configurable number of expected inputs
//!   for flexibility
//!
//! # Integration with StreamWeave
//!
//! [`Synchronize`] implements the [`InputRouter`] trait and can be used in any
//! StreamWeave graph. It synchronizes items from multiple input streams,
//! enabling barrier synchronization patterns.

use crate::graph::router::InputRouter;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::mpsc;

/// Input router that synchronizes multiple input streams, waiting for all streams
/// to have items available before proceeding.
///
/// This router collects items from all input ports and emits them together when
/// all ports have items available. It acts as a synchronization barrier, ensuring
/// that processing only proceeds when all input streams are ready.
///
/// # Behavior
///
/// When all input streams have items available, the router emits one item (the first
/// item from the first stream). If any stream doesn't have an item ready, the router
/// waits and retries until all streams have items.
///
/// # Port Names
///
/// The router expects ports named `in`, `in_1`, `in_2`, etc., based on the number
/// of expected inputs.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::nodes::Synchronize;
/// use streamweave::graph::router::InputRouter;
///
/// // Create a synchronize router that waits for 3 input streams
/// let sync = Synchronize::<i32>::new(3);
///
/// // This router expects 3 input ports: "in", "in_1", "in_2"
/// let port_names = sync.expected_port_names();
/// assert_eq!(port_names, vec!["in", "in_1", "in_2"]);
/// ```
pub struct Synchronize<T> {
  /// Number of expected inputs
  expected_inputs: usize,
  /// Phantom data for type parameter
  _phantom: PhantomData<T>,
}

impl<T> Synchronize<T> {
  /// Creates a new `Synchronize` input router.
  ///
  /// # Arguments
  ///
  /// * `expected_inputs` - Number of input streams to synchronize
  ///
  /// # Returns
  ///
  /// A new `Synchronize` instance.
  pub fn new(expected_inputs: usize) -> Self {
    Self {
      expected_inputs,
      _phantom: PhantomData,
    }
  }
}

#[async_trait]
impl<T> InputRouter<T> for Synchronize<T>
where
  T: Send + Sync + Clone + 'static,
{
  async fn route_streams(
    &mut self,
    streams: Vec<(String, Pin<Box<dyn Stream<Item = T> + Send>>)>,
  ) -> Pin<Box<dyn Stream<Item = T> + Send>> {
    if streams.len() != self.expected_inputs {
      // Return empty stream if input count doesn't match
      return Box::pin(futures::stream::empty());
    }

    // Create channels for collecting items from each stream
    let mut receivers: Vec<_> = streams
      .into_iter()
      .map(|(_, stream)| {
        let (tx, rx) = mpsc::channel(16);
        // Spawn task to forward stream items
        tokio::spawn(async move {
          let mut s = stream;
          while let Some(item) = s.next().await {
            let _ = tx.send(item).await;
          }
        });
        rx
      })
      .collect();

    // Collect items from all streams and emit when all have items
    let expected = self.expected_inputs;
    Box::pin(async_stream::stream! {
      loop {
        let mut items = Vec::new();
        let mut all_ready = true;

        // Try to receive from all streams
        for rx in &mut receivers {
          match rx.try_recv() {
            Ok(item) => items.push(item),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
              all_ready = false;
              break;
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
              // Stream ended
              return;
            }
          }
        }

        if all_ready && items.len() == expected {
          // All streams have items - emit the first one as representative
          // (in a real implementation, you might want to emit a tuple or aggregate)
          if let Some(item) = items.into_iter().next() {
            yield item;
          }
        } else if !all_ready {
          // Not all ready - wait a bit and try again
          tokio::time::sleep(Duration::from_millis(1)).await;
        } else {
          // Stream ended
          break;
        }
      }
    })
  }

  fn expected_port_names(&self) -> Vec<String> {
    (0..self.expected_inputs)
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
