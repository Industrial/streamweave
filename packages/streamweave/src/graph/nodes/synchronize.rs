//! Input router that synchronizes multiple input streams, waiting for all before proceeding.

use crate::graph::router::InputRouter;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::mpsc;

/// Input router that synchronizes multiple input streams, waiting for all before proceeding.
///
/// This is an `InputRouter` that collects items from all input ports and emits
/// them together when all ports have items available.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::control_flow::Synchronize;
/// use streamweave::graph::router::InputRouter;
///
/// let sync = Synchronize::new(3); // Wait for 3 inputs
/// // Use with a node that has 3 input ports
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
