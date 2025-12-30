//! # Broadcast Router
//!
//! This module provides a BroadcastRouter that clones each item and sends it
//! to all output ports. This implements the broadcast pattern for OutputRouter.

use crate::router::OutputRouter;
use async_trait::async_trait;
use futures::Stream;
use std::marker::PhantomData;
use std::pin::Pin;

/// A router that broadcasts each item to all output ports.
///
/// This router clones each item from the input stream and sends a copy to
/// every output port. All downstream nodes receive all items.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::routers::BroadcastRouter;
/// use futures::stream;
/// use std::pin::Pin;
///
/// let mut router = BroadcastRouter::new(vec![0, 1, 2]);
/// let input_stream: Pin<Box<dyn Stream<Item = i32> + Send>> =
///     Box::pin(stream::iter(vec![1, 2, 3]));
///
/// let output_streams = router.route_stream(input_stream).await;
/// // Each stream receives: [1, 2, 3]
/// ```
pub struct BroadcastRouter<O> {
  /// The output port indices this router manages
  output_ports: Vec<usize>,
  /// Phantom data to consume the type parameter
  _phantom: PhantomData<O>,
}

impl<O> BroadcastRouter<O> {
  /// Creates a new BroadcastRouter with the specified output ports.
  ///
  /// # Arguments
  ///
  /// * `output_ports` - Vector of port indices to broadcast to
  ///
  /// # Returns
  ///
  /// A new `BroadcastRouter` instance.
  pub fn new(output_ports: Vec<usize>) -> Self {
    Self {
      output_ports,
      _phantom: PhantomData,
    }
  }
}

#[async_trait]
impl<O> OutputRouter<O> for BroadcastRouter<O>
where
  O: Send + Sync + Clone + 'static,
{
  async fn route_stream(
    &mut self,
    stream: Pin<Box<dyn Stream<Item = O> + Send>>,
  ) -> Vec<(usize, Pin<Box<dyn Stream<Item = O> + Send>>)> {
    // For broadcast, we need to clone each item to all ports
    // We'll use individual mpsc channels for each port
    use futures::stream::StreamExt as _;
    use tokio::sync::mpsc;

    if self.output_ports.is_empty() {
      return Vec::new();
    }

    // Create channels for each output port
    let mut senders = Vec::new();
    let mut receivers = Vec::new();

    for _ in &self.output_ports {
      let (tx, rx) = mpsc::channel(16);
      senders.push(tx);
      receivers.push(rx);
    }

    // Spawn a task to read from input and broadcast to all receivers
    let mut input_stream = stream;
    let senders_clone = senders.clone();
    tokio::spawn(async move {
      while let Some(item) = input_stream.next().await {
        // Clone and send to all receivers
        for sender in &senders_clone {
          if sender.send(item.clone()).await.is_err() {
            // Receiver dropped, continue to next sender
          }
        }
      }
    });

    // Create streams from receivers
    let mut output_streams: Vec<(usize, Pin<Box<dyn Stream<Item = O> + Send>>)> = Vec::new();
    for &port in self.output_ports.iter() {
      let mut rx = receivers.remove(0);
      let stream = Box::pin(async_stream::stream! {
        while let Some(item) = rx.recv().await {
          yield item;
        }
      });
      output_streams.push((port, stream));
    }

    output_streams
  }

  fn output_ports(&self) -> Vec<usize> {
    self.output_ports.clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_broadcast_router() {
    let mut router = BroadcastRouter::new(vec![0, 1, 2]);
    let input_stream: Pin<Box<dyn Stream<Item = i32> + Send>> =
      Box::pin(stream::iter(vec![1, 2, 3]));

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
    assert_eq!(router.output_ports(), vec![0, 1, 2]);
  }
}
