//! # Router Traits
//!
//! This module provides router traits for handling fan-in and fan-out patterns
//! in the graph. Routers manage the flow of data between nodes, supporting
//! multiple input/output ports per node.
//!
//! ## InputRouter
//!
//! The `InputRouter` trait handles fan-in patterns (multiple input streams → single node).
//! It takes multiple input streams, each tagged with a port name, and merges them
//! into a single stream according to the router's strategy.
//!
//! ## Example
//!
//! ```rust
//! use crate::graph::router::InputRouter;
//! use futures::Stream;
//! use std::pin::Pin;
//!
//! // A simple merge-all router (stateless)
//! struct MergeAllRouter<I> {
//!     expected_port_names: Vec<String>,
//! }
//!
//! #[async_trait::async_trait]
//! impl<I> InputRouter<I> for MergeAllRouter<I>
//! where
//!     I: Send + Sync + 'static,
//! {
//!     async fn route_streams(
//!         &mut self,
//!         streams: Vec<(String, Pin<Box<dyn Stream<Item = I> + Send>>)>,
//!     ) -> Pin<Box<dyn Stream<Item = I> + Send>> {
//!         // Merge all streams using select_all
//!         Box::pin(futures::stream::select_all(
//!             streams.into_iter().map(|(_, stream)| stream)
//!         ))
//!     }
//!
//!     fn expected_port_names(&self) -> Vec<String> {
//!         self.expected_port_names.clone()
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
/// use crate::graph::router::InputRouter;
/// use futures::Stream;
/// use std::pin::Pin;
///
/// // Stateful round-robin router
/// struct RoundRobinRouter<I> {
///     expected_port_names: Vec<String>,
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
///         streams: Vec<(String, Pin<Box<dyn Stream<Item = I> + Send>>)>,
///     ) -> Pin<Box<dyn Stream<Item = I> + Send>> {
///         // Round-robin implementation with state
///         // ...
///         # Box::pin(futures::stream::empty())
///     }
///
///     fn expected_port_names(&self) -> Vec<String> {
///         self.expected_port_names.clone()
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
  /// * `streams` - Vector of (port_name, stream) tuples representing
  ///   inputs from different ports. The port name identifies the specific
  ///   input port on the node.
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
    streams: Vec<(String, Pin<Box<dyn Stream<Item = I> + Send>>)>,
  ) -> Pin<Box<dyn Stream<Item = I> + Send>>;

  /// Returns the port names this router expects to receive.
  ///
  /// This method is used for validation during graph construction and
  /// to ensure that all required input ports are connected.
  ///
  /// # Returns
  ///
  /// A vector of port names (String) that this router expects to receive
  /// input from.
  ///
  /// # Example
  ///
  /// If a router expects inputs from ports named "in_a" and "in_b",
  /// this method would return `vec!["in_a".to_string(), "in_b".to_string()]`.
  fn expected_port_names(&self) -> Vec<String>;
}

/// Error type for router operations.
///
/// This enum represents errors that can occur during router operations,
/// including invalid port configurations, stream errors, and configuration issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterError {
  /// Invalid port name provided
  InvalidPort {
    /// The invalid port name
    port_name: String,
    /// The expected port names
    expected: Vec<String>,
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
      RouterError::InvalidPort {
        port_name,
        expected,
      } => {
        write!(
          f,
          "Invalid port name: '{}' (expected one of: {:?})",
          port_name, expected
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

/// Trait for routing a single output stream into multiple output streams.
///
/// `OutputRouter` handles fan-out patterns where a single output stream from
/// a node (producer or transformer) is distributed to multiple output ports
/// for consumption by downstream nodes.
///
/// ## Routing Strategies
///
/// Different router implementations can use various strategies:
/// - **Broadcast**: Clone each item and send to all ports (all consumers see all data)
/// - **Round-Robin**: Distribute items in round-robin fashion (each item to one port)
/// - **Key-Based**: Route items based on a key function (partitioning)
/// - **Priority**: Route items based on priority (high-priority items to specific ports)
///
/// ## Port Indices
///
/// Port indices correspond to positions in the node's `Outputs` port tuple.
/// For example, if a node has `Outputs = (i32, String)`, then:
/// - Port 0 outputs `i32` items
/// - Port 1 outputs `String` items
///
/// ## Broadcast vs Distribution
///
/// Routers can implement two main patterns:
/// - **Broadcast**: Each item is cloned and sent to all output ports
///   - Requires `O: Clone`
///   - All downstream nodes receive all items
///   - Useful for: logging + processing, parallel processing, multiple views
/// - **Distribution**: Each item goes to exactly one port
///   - No cloning required
///   - Each downstream node receives a subset of items
///   - Useful for: load balancing, partitioning, priority routing
///
/// ## State Management
///
/// Routers can be either stateless or stateful:
/// - **Stateless**: Pure routing logic, no internal state (e.g., broadcast-all)
/// - **Stateful**: Maintain state between routing operations (e.g., round-robin counter)
///
/// ## Integration with BroadcastTransformer
///
/// Broadcast routers can leverage the existing `BroadcastTransformer` internally,
/// following functional programming principles by reusing existing code rather
/// than reimplementing broadcast logic.
///
/// ## Example
///
/// ```rust
/// use crate::graph::router::OutputRouter;
/// use futures::Stream;
/// use std::pin::Pin;
///
/// // Broadcast router (clones to all ports)
/// struct BroadcastRouter<O> {
///     output_port_names: Vec<String>,
/// }
///
/// #[async_trait::async_trait]
/// impl<O> OutputRouter<O> for BroadcastRouter<O>
/// where
///     O: Send + Sync + Clone + 'static,
/// {
///     async fn route_stream(
///         &mut self,
///         stream: Pin<Box<dyn Stream<Item = O> + Send>>,
///     ) -> Vec<(String, Pin<Box<dyn Stream<Item = O> + Send>>)> {
///         // Clone items and send to all ports
///         // Can leverage BroadcastTransformer internally
///         // ...
///         # vec![]
///     }
///
///     fn output_port_names(&self) -> Vec<String> {
///         self.output_port_names.clone()
///     }
/// }
///
/// // Round-robin router (distributes to one port)
/// struct RoundRobinRouter<O> {
///     output_port_names: Vec<String>,
///     next_port: usize,  // State maintained across calls
/// }
///
/// #[async_trait::async_trait]
/// impl<O> OutputRouter<O> for RoundRobinRouter<O>
/// where
///     O: Send + Sync + 'static,
/// {
///     async fn route_stream(
///         &mut self,
///         stream: Pin<Box<dyn Stream<Item = O> + Send>>,
///     ) -> Vec<(String, Pin<Box<dyn Stream<Item = O> + Send>>)> {
///         // Distribute items in round-robin fashion
///         // Each item goes to exactly one port
///         // ...
///         # vec![]
///     }
///
///     fn output_port_names(&self) -> Vec<String> {
///         self.output_port_names.clone()
///     }
/// }
/// ```
#[async_trait]
pub trait OutputRouter<O>: Send + Sync
where
  O: Send + Sync + 'static,
{
  /// Routes a single input stream into multiple output streams.
  ///
  /// This method takes a single input stream and distributes its items
  /// to multiple output ports according to the router's strategy.
  ///
  /// # Arguments
  ///
  /// * `stream` - The input stream to route. Items from this stream will
  ///   be distributed to output ports based on the router's strategy.
  ///
  /// # Returns
  ///
  /// A vector of (port_name, stream) tuples, one for each output port.
  /// The number of streams matches the number of output ports this router
  /// manages. Each stream is independent and can be consumed by different
  /// downstream nodes in parallel.
  ///
  /// # Routing Strategies
  ///
  /// Different router implementations use different strategies:
  /// - **Broadcast**: Clone each item and send to all ports
  ///   - All ports receive all items
  ///   - Requires `O: Clone`
  /// - **Round-Robin**: Distribute items in round-robin fashion
  ///   - Each item goes to exactly one port
  ///   - Cycles through ports: 0, 1, 2, ..., 0, 1, 2, ...
  /// - **Key-Based**: Route items based on a key extraction function
  ///   - Items with same key go to same port
  ///   - Useful for partitioning data
  /// - **Priority**: Route items based on priority
  ///   - High-priority items to specific ports
  ///
  /// # Behavior
  ///
  /// The router implementation determines how items are distributed:
  /// - Broadcast routers: Clone items and send to all ports
  /// - Distribution routers: Route each item to exactly one port
  ///
  /// Each output stream is independent and can be consumed at different rates
  /// by downstream nodes. For broadcast routers, slow consumers may cause
  /// buffering or backpressure.
  ///
  /// # Errors
  ///
  /// Implementations should handle stream errors gracefully according to
  /// the router's error strategy. Streams that error may be propagated to
  /// all output streams (broadcast) or only to the affected port (distribution).
  async fn route_stream(
    &mut self,
    stream: Pin<Box<dyn Stream<Item = O> + Send>>,
  ) -> Vec<(String, Pin<Box<dyn Stream<Item = O> + Send>>)>;

  /// Returns the port names this router will output to.
  ///
  /// This method is used for validation during graph construction and
  /// to ensure that all required output ports are available for connection.
  ///
  /// # Returns
  ///
  /// A vector of port names (String) that this router will output to.
  ///
  /// # Example
  ///
  /// If a router outputs to ports named "out_a" and "out_b",
  /// this method would return `vec!["out_a".to_string(), "out_b".to_string()]`.
  fn output_port_names(&self) -> Vec<String>;
}
