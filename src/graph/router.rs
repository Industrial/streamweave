//! # Router Traits
//!
//! This module provides router traits for handling fan-in and fan-out patterns
//! in the graph. Routers manage the flow of data between nodes, supporting
//! multiple input/output ports per node.
//!
//! ## InputRouter
//!
//! The `InputRouter` trait handles fan-in patterns (multiple input streams → single node).
//! It takes multiple input streams, each tagged with a port index, and merges them
//! into a single stream according to the router's strategy.
//!
//! ## Example
//!
//! ```rust
//! use streamweave::graph::router::InputRouter;
//! use futures::Stream;
//! use std::pin::Pin;
//!
//! // A simple merge-all router (stateless)
//! struct MergeAllRouter<I> {
//!     expected_ports: Vec<usize>,
//! }
//!
//! #[async_trait::async_trait]
//! impl<I> InputRouter<I> for MergeAllRouter<I>
//! where
//!     I: Send + Sync + 'static,
//! {
//!     async fn route_streams(
//!         &mut self,
//!         streams: Vec<(usize, Pin<Box<dyn Stream<Item = I> + Send>>)>,
//!     ) -> Pin<Box<dyn Stream<Item = I> + Send>> {
//!         // Merge all streams using select_all
//!         Box::pin(futures::stream::select_all(
//!             streams.into_iter().map(|(_, stream)| stream)
//!         ))
//!     }
//!
//!     fn expected_ports(&self) -> Vec<usize> {
//!         self.expected_ports.clone()
//!     }
//! }
//! ```

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Trait for routing multiple input streams into a single output stream.
///
/// `InputRouter` handles fan-in patterns where multiple input streams from
/// different ports are merged into a single stream for processing by a node
/// (transformer or consumer).
///
/// ## Routing Strategies
///
/// Different router implementations can use various strategies:
/// - **Merge**: Combine all inputs into a single stream (interleaved)
/// - **Round-Robin**: Cycle through inputs, taking one item from each in turn
/// - **Priority**: Process higher-priority inputs first
/// - **Sequential**: Process inputs one at a time, exhausting each before moving to next
///
/// ## Port Indices
///
/// Port indices correspond to positions in the node's `Inputs` port tuple.
/// For example, if a node has `Inputs = (i32, String)`, then:
/// - Port 0 expects `i32` items
/// - Port 1 expects `String` items
///
/// ## State Management
///
/// Routers can be either stateless or stateful:
/// - **Stateless**: Pure routing logic, no internal state (e.g., merge-all)
/// - **Stateful**: Maintain state between routing operations (e.g., round-robin counter)
///
/// ## Example
///
/// ```rust
/// use streamweave::graph::router::InputRouter;
/// use futures::Stream;
/// use std::pin::Pin;
///
/// // Stateful round-robin router
/// struct RoundRobinRouter<I> {
///     expected_ports: Vec<usize>,
///     next_port: usize,  // State maintained across calls
/// }
///
/// #[async_trait::async_trait]
/// impl<I> InputRouter<I> for RoundRobinRouter<I>
/// where
///     I: Send + Sync + 'static,
/// {
///     async fn route_streams(
///         &mut self,
///         streams: Vec<(usize, Pin<Box<dyn Stream<Item = I> + Send>>)>,
///     ) -> Pin<Box<dyn Stream<Item = I> + Send>> {
///         // Round-robin implementation with state
///         // ...
///         # Box::pin(futures::stream::empty())
///     }
///
///     fn expected_ports(&self) -> Vec<usize> {
///         self.expected_ports.clone()
///     }
/// }
/// ```
#[async_trait]
pub trait InputRouter<I>: Send + Sync
where
  I: Send + Sync + 'static,
{
  /// Routes multiple input streams into a single output stream.
  ///
  /// This method takes a collection of input streams, each tagged with its
  /// port index, and merges them into a single stream according to the
  /// router's strategy.
  ///
  /// # Arguments
  ///
  /// * `streams` - Vector of (port_index, stream) tuples representing
  ///   inputs from different ports. The port index corresponds to the
  ///   position in the node's `Inputs` port tuple.
  ///
  /// # Returns
  ///
  /// A single merged stream containing items from all input streams,
  /// ordered according to the router's strategy.
  ///
  /// # Behavior
  ///
  /// The router implementation determines how items from different streams
  /// are interleaved:
  /// - Merge routers: Fair interleaving (whichever stream has data ready)
  /// - Round-robin routers: Cycle through streams in order
  /// - Priority routers: Process higher-priority streams first
  /// - Sequential routers: Exhaust each stream before moving to next
  ///
  /// # Errors
  ///
  /// Implementations should handle stream errors gracefully according to
  /// the router's error strategy. Streams that error may be skipped,
  /// retried, or cause the entire routing operation to fail.
  async fn route_streams(
    &mut self,
    streams: Vec<(usize, Pin<Box<dyn Stream<Item = I> + Send>>)>,
  ) -> Pin<Box<dyn Stream<Item = I> + Send>>;

  /// Returns the port indices this router expects to receive.
  ///
  /// This method is used for validation during graph construction and
  /// to ensure that all required input ports are connected.
  ///
  /// # Returns
  ///
  /// A vector of port indices (usize) that this router expects to receive
  /// input from. These indices correspond to positions in the node's
  /// `Inputs` port tuple.
  ///
  /// # Example
  ///
  /// If a node has `Inputs = (i32, String, bool)`, and the router expects
  /// inputs from ports 0 and 2, this method would return `vec![0, 2]`.
  fn expected_ports(&self) -> Vec<usize>;
}

/// Error type for router operations.
///
/// This enum represents errors that can occur during router operations,
/// including invalid port configurations, stream errors, and configuration issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterError {
  /// Invalid port index provided
  InvalidPort {
    /// The invalid port index
    port: usize,
    /// The expected port indices
    expected: Vec<usize>,
  },
  /// Error during stream processing
  StreamError {
    /// Description of the stream error
    message: String,
  },
  /// Configuration error
  ConfigurationError {
    /// Description of the configuration error
    message: String,
  },
}

impl std::fmt::Display for RouterError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RouterError::InvalidPort { port, expected } => {
        write!(
          f,
          "Invalid port index: {} (expected one of: {:?})",
          port, expected
        )
      }
      RouterError::StreamError { message } => {
        write!(f, "Stream error: {}", message)
      }
      RouterError::ConfigurationError { message } => {
        write!(f, "Configuration error: {}", message)
      }
    }
  }
}

impl std::error::Error for RouterError {}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::{stream, StreamExt};

  // Test helper: Simple stateless merge router
  struct TestMergeRouter {
    expected_ports: Vec<usize>,
  }

  #[async_trait]
  impl<I> InputRouter<I> for TestMergeRouter
  where
    I: Send + Sync + 'static,
  {
    async fn route_streams(
      &mut self,
      streams: Vec<(usize, Pin<Box<dyn Stream<Item = I> + Send>>)>,
    ) -> Pin<Box<dyn Stream<Item = I> + Send>> {
      Box::pin(futures::stream::select_all(
        streams.into_iter().map(|(_, stream)| stream),
      ))
    }

    fn expected_ports(&self) -> Vec<usize> {
      self.expected_ports.clone()
    }
  }

  #[tokio::test]
  async fn test_input_router_trait() {
    let mut router = TestMergeRouter {
      expected_ports: vec![0, 1],
    };

    // Create test streams
    let stream1: Pin<Box<dyn Stream<Item = i32> + Send>> =
      Box::pin(stream::iter(vec![1, 2, 3]));
    let stream2: Pin<Box<dyn Stream<Item = i32> + Send>> =
      Box::pin(stream::iter(vec![4, 5, 6]));

    let streams = vec![(0, stream1), (1, stream2)];
    let mut merged = router.route_streams(streams).await;

    // Collect results
    let mut results = Vec::new();
    while let Some(item) = merged.next().await {
      results.push(item);
    }

    // Should have all items (order may vary due to select_all)
    assert_eq!(results.len(), 6);
    assert!(results.contains(&1));
    assert!(results.contains(&2));
    assert!(results.contains(&3));
    assert!(results.contains(&4));
    assert!(results.contains(&5));
    assert!(results.contains(&6));
  }

  #[test]
  fn test_expected_ports() {
    let router = TestMergeRouter {
      expected_ports: vec![0, 1, 2],
    };

    let ports = router.expected_ports();
    assert_eq!(ports, vec![0, 1, 2]);
  }

  #[test]
  fn test_router_error_display() {
    let error = RouterError::InvalidPort {
      port: 5,
      expected: vec![0, 1, 2],
    };
    assert_eq!(
      error.to_string(),
      "Invalid port index: 5 (expected one of: [0, 1, 2])"
    );

    let error = RouterError::StreamError {
      message: "Connection lost".to_string(),
    };
    assert_eq!(error.to_string(), "Stream error: Connection lost");

    let error = RouterError::ConfigurationError {
      message: "Invalid strategy".to_string(),
    };
    assert_eq!(error.to_string(), "Configuration error: Invalid strategy");
  }
}

