//! # Round-Robin Router
//!
//! This module provides a RoundRobinRouter that distributes items in round-robin
//! fashion to output ports. Each item goes to exactly one port.

use crate::graph::router::OutputRouter;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::pin::Pin;

/// A router that distributes items in round-robin fashion.
///
/// This router cycles through output ports, sending each item to exactly one port.
/// Items are distributed: port 0, port 1, port 2, ..., port 0, port 1, ...
///
/// # Example
///
/// ```rust
/// use streamweave::graph::routers::RoundRobinRouter;
/// use futures::stream;
/// use std::pin::Pin;
///
/// let mut router = RoundRobinRouter::new(vec![0, 1, 2]);
/// let input_stream: Pin<Box<dyn Stream<Item = i32> + Send>> =
///     Box::pin(stream::iter(vec![1, 2, 3, 4, 5, 6]));
///
/// let output_streams = router.route_stream(input_stream).await;
/// // Stream 0 receives: [1, 4]
/// // Stream 1 receives: [2, 5]
/// // Stream 2 receives: [3, 6]
/// ```
pub struct RoundRobinRouter<O> {
  /// The output port indices this router manages
  output_ports: Vec<usize>,
  /// The next port to send an item to (state)
  next_port: usize,
}

impl<O> RoundRobinRouter<O> {
  /// Creates a new RoundRobinRouter with the specified output ports.
  ///
  /// # Arguments
  ///
  /// * `output_ports` - Vector of port indices to distribute to
  ///
  /// # Returns
  ///
  /// A new `RoundRobinRouter` instance.
  pub fn new(output_ports: Vec<usize>) -> Self {
    Self {
      output_ports,
      next_port: 0,
    }
  }
}

#[async_trait]
impl<O> OutputRouter<O> for RoundRobinRouter<O>
where
  O: Send + Sync + 'static,
{
  async fn route_stream(
    &mut self,
    stream: Pin<Box<dyn Stream<Item = O> + Send>>,
  ) -> Vec<(usize, Pin<Box<dyn Stream<Item = O> + Send>>)> {
    if self.output_ports.is_empty() {
      return Vec::new();
    }

    // Create channels for each output port
    use tokio::sync::mpsc;
    let mut senders = Vec::new();
    let mut receivers = Vec::new();

    for _ in &self.output_ports {
      let (tx, rx) = mpsc::channel(16);
      senders.push(tx);
      receivers.push(rx);
    }

    // Spawn task to distribute items round-robin
    let mut input_stream = stream;
    let mut senders_clone = senders.clone();
    let mut next_port = self.next_port;
    let num_ports = self.output_ports.len();

    tokio::spawn(async move {
      while let Some(item) = input_stream.next().await {
        // Send to current port
        let port_idx = next_port % num_ports;
        if let Err(_) = senders_clone[port_idx].send(item).await {
          // Receiver dropped, continue to next port
        }
        next_port = (next_port + 1) % num_ports;
      }
    });

    // Update state for next call
    self.next_port = (self.next_port + receivers.len()) % num_ports.max(1);

    // Create streams from receivers
    let mut output_streams = Vec::new();
    for (i, &port) in self.output_ports.iter().enumerate() {
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
  async fn test_round_robin_router() {
    let mut router = RoundRobinRouter::new(vec![0, 1, 2]);
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

    // Items should be distributed round-robin
    // Stream 0: [1, 4]
    // Stream 1: [2, 5]
    // Stream 2: [3, 6]
    assert_eq!(results[0], vec![1, 4]);
    assert_eq!(results[1], vec![2, 5]);
    assert_eq!(results[2], vec![3, 6]);
  }

  #[test]
  fn test_round_robin_output_ports() {
    let router = RoundRobinRouter::<i32>::new(vec![0, 1, 2]);
    assert_eq!(router.output_ports(), vec![0, 1, 2]);
  }
}

