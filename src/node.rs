//! # Unified Node Architecture - Pure Stream Implementation
//!
//! This module defines the core `Node` trait for StreamWeave's unified streaming architecture.
//! All streaming operations (sources, transforms, sinks) are implemented as Nodes with
//! zero-copy data passing via `Arc<T>`.
//!
//! ## Design Principles
//!
//! - **Unified Interface**: One `Node` trait for all operations (sources, transforms, sinks)
//! - **Zero-Copy**: All data flows as `Arc<T>` for efficient sharing
//! - **Stream-Based**: Nodes consume and produce streams - no channels exposed
//! - **Pure Functional**: Stream composition enables clean, functional programming style
//!
//! ## Node Types
//!
//! Nodes can have different port configurations:
//!
//! - **Source Nodes**: 0 inputs, 1+ outputs (generate data)
//! - **Transform Nodes**: 1+ inputs, 1+ outputs (process data)
//! - **Sink Nodes**: 1+ inputs, 0 outputs (consume data)
//!
//! ## Zero-Copy Architecture
//!
//! All data flows as `Arc<T>`:
//!
//! - **Single Output**: Data moved directly (zero-cost move of Arc)
//! - **Fan-Out**: Data shared via `Arc::clone()` (atomic refcount increment, ~1-2ns)
//! - **No Serialization**: Direct in-process passing, no overhead
//! - **No Wrapping**: Raw payload types, no message envelopes
//!
//! ## Stream-Based Port System
//!
//! All nodes work with streams:
//!
//! - Input ports: `HashMap<String, InputStream>` where `InputStream = Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>`
//! - Output ports: `HashMap<String, OutputStream>` where `OutputStream = Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>`
//! - Nodes consume input streams and produce output streams
//! - Graph execution engine connects streams between nodes
//! - Channels are used internally for backpressure, but never exposed to nodes
//!
//! ## Execution Model
//!
//! The graph execution engine:
//! 1. Collects input streams for each node from connected upstream nodes
//! 2. Calls `execute(inputs)` which returns output streams
//! 3. Connects output streams to downstream nodes' input streams
//! 4. Drives all streams to completion
//!
//! Nodes process streams functionally - no channels, no Mutex locking, pure stream composition.
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::node::Node;
//! use std::sync::Arc;
//! use std::collections::HashMap;
//! use tokio_stream::StreamExt;
//! use async_stream::stream;
//!
//! // Transform node that doubles integers
//! struct DoubleNode {
//!     name: String,
//! }
//!
//! impl Node for DoubleNode {
//!     fn name(&self) -> &str { &self.name }
//!     fn set_name(&mut self, name: &str) { self.name = name.to_string(); }
//!     fn input_port_names(&self) -> &[String] { &["in".to_string()] }
//!     fn output_port_names(&self) -> &[String] { &["out".to_string()] }
//!     fn has_input_port(&self, name: &str) -> bool { name == "in" }
//!     fn has_output_port(&self, name: &str) -> bool { name == "out" }
//!
//!     fn execute(
//!         &self,
//!         mut inputs: InputStreams,
//!     ) -> Pin<Box<dyn Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>> {
//!         Box::pin(async move {
//!             let input_stream = inputs.remove("in")
//!                 .ok_or("Missing 'in' input")?;
//!
//!             let output_stream: OutputStream = Box::pin(stream! {
//!                 let mut input = input_stream;
//!                 while let Some(item) = input.next().await {
//!                     if let Ok(arc_i32) = item.clone().downcast::<i32>() {
//!                         let doubled = *arc_i32 * 2;
//!                         yield Arc::new(doubled) as Arc<dyn Any + Send + Sync>;
//!                     }
//!                 }
//!             });
//!
//!             let mut outputs = HashMap::new();
//!             outputs.insert("out".to_string(), output_stream);
//!             Ok(outputs)
//!         })
//!     }
//! }
//! ```

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;

/// Type alias for input streams.
///
/// Input streams are pinned, boxed streams that yield `Arc<dyn Any + Send + Sync>` items.
/// Nodes consume these streams to process data.
pub type InputStream = Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>;

/// Type alias for output streams.
///
/// Output streams are pinned, boxed streams that yield `Arc<dyn Any + Send + Sync>` items.
/// Nodes produce these streams as their output.
pub type OutputStream = Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>;

/// Type alias for a collection of input streams, keyed by port name.
pub type InputStreams = HashMap<String, InputStream>;

/// Type alias for a collection of output streams, keyed by port name.
pub type OutputStreams = HashMap<String, OutputStream>;

/// Error type for node execution operations.
pub type NodeExecutionError = Box<dyn std::error::Error + Send + Sync>;

/// The unified Node trait for all streaming operations.
///
/// All nodes in StreamWeave implement this trait, whether they are sources,
/// transforms, or sinks. The `execute` method consumes input streams and
/// produces output streams.
///
/// # Stream-Based Architecture
///
/// Nodes work purely with streams:
///
/// - **Input**: `HashMap<String, InputStream>` - named input streams
/// - **Output**: `HashMap<String, OutputStream>` - named output streams
/// - **No Channels**: Channels are used internally by the graph execution engine
///   for backpressure, but nodes never see them
/// - **No Mutex**: Streams are async-native, no locking needed
///
/// # Execution Model
///
/// 1. Graph execution engine collects input streams for a node from upstream nodes
/// 2. Calls `execute(inputs)` with those streams
/// 3. Node processes streams and returns output streams
/// 4. Graph execution engine connects output streams to downstream nodes
///
/// # Port Naming Convention
///
/// - Single input port: `"in"`
/// - Single output port: `"out"`
/// - Multiple ports: `"in_0"`, `"in_1"`, `"out_0"`, `"out_1"`, etc.
/// - Mandatory ports: `"configuration"` (input), `"error"` (output)
///
/// # Zero-Copy Execution
///
/// Data is passed as `Arc<T>` wrapped in `Arc<dyn Any + Send + Sync>`:
///
/// - **Single output**: `Arc::clone()` is cheap (atomic increment, ~1-2ns)
/// - **Fan-out**: Multiple `Arc::clone()` calls share the same data
/// - **No serialization**: Direct in-process passing
/// - **No wrapping**: Raw payload types, no message envelopes
///
/// # Type Safety
///
/// Nodes are responsible for downcasting to their expected types when receiving
/// data from input streams.
#[async_trait]
pub trait Node: Send + Sync {
  /// Returns the name of the node.
  fn name(&self) -> &str;

  /// Sets the name of the node.
  fn set_name(&mut self, name: &str);

  /// Returns the names of all input ports.
  ///
  /// # Returns
  ///
  /// A slice of strings containing the names of all input ports, in order.
  /// Empty slice for source nodes (nodes with no inputs).
  fn input_port_names(&self) -> &[String];

  /// Returns the names of all output ports.
  ///
  /// # Returns
  ///
  /// A slice of strings containing the names of all output ports, in order.
  /// Empty slice for sink nodes (nodes with no outputs).
  fn output_port_names(&self) -> &[String];

  /// Checks if this node has an input port with the given name.
  fn has_input_port(&self, name: &str) -> bool;

  /// Checks if this node has an output port with the given name.
  fn has_output_port(&self, name: &str) -> bool;

  /// Executes the node's logic.
  ///
  /// This method consumes input streams and produces output streams. The graph
  /// execution engine calls this method with the appropriate input streams
  /// collected from upstream nodes.
  ///
  /// # Arguments
  ///
  /// * `inputs` - A HashMap of input streams, keyed by port name. The node
  ///   should remove the streams it needs from this HashMap. Streams that are
  ///   not removed will be dropped.
  ///
  /// # Returns
  ///
  /// A HashMap of output streams, keyed by port name. The graph execution engine
  /// will connect these streams to downstream nodes' input streams.
  ///
  /// # Node Types
  ///
  /// - **Source nodes**: `inputs` will be empty. Node generates data and returns output streams.
  /// - **Transform nodes**: `inputs` contains streams from upstream nodes. Node processes
  ///   data and returns output streams.
  /// - **Sink nodes**: `inputs` contains streams from upstream nodes. Node consumes data
  ///   and returns empty HashMap (or streams that are consumed internally).
  ///
  /// # Zero-Copy Execution
  ///
  /// Data is passed as `Arc<T>` wrapped in `Arc<dyn Any + Send + Sync>`:
  ///
  /// - **Single output**: `Arc::clone()` is cheap (atomic increment, ~1-2ns)
  /// - **Fan-out**: Multiple `Arc::clone()` calls share the same data
  /// - **No serialization**: Direct in-process passing
  /// - **No wrapping**: Raw payload types, no message envelopes
  ///
  /// # Type Safety
  ///
  /// Nodes are responsible for downcasting to their expected types when receiving
  /// data from input streams.
  fn execute(
    &self,
    inputs: InputStreams,
  ) -> Pin<Box<dyn Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>>;
}
