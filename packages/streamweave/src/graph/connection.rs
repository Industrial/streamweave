//! # Connection Types
//!
//! This module provides connection types that validate port compatibility at compile time
//! using trait bounds and const generics. Connections reference nodes by type and ports
//! by compile-time indices for type checking, while runtime uses port names.
//!
//! ## Note on Port Indices vs Port Names
//!
//! The const generic parameters (`const N: usize`) are used for compile-time type checking
//! only. At runtime, all ports are identified by string names. The `GraphBuilder::connect`
//! method converts port indices to port names when creating connections.
//!
//! ## Message<T> and Port Types
//!
//! Port types are specified using raw types (e.g., `i32`, `String`), not `Message<T>`.
//! This keeps the user-facing API clean and intuitive. Internally, all data flowing through
//! connections is wrapped in `Message<T>` for traceability and metadata preservation.
//!
//! The connection validation checks that raw types are compatible (e.g., `i32` output
//! can connect to `i32` input). The nodes automatically wrap outputs in `Message<T>`
//! before sending and unwrap `Message<T>` from inputs before processing.
//!
//! ## Example
//!
//! ```rust
//! use streamweave::graph::connection::Connection;
//! use streamweave::graph::node::{ProducerNode, TransformerNode};
//! use streamweave_VecProducer;
//! use streamweave_transformers::MapTransformer;
//!
//! // Type-safe connection validated at compile time
//! // Note: Port indices (0, 0) are for compile-time validation only.
//! // At runtime, ports are identified by names (e.g., "out", "in").
//! // Port types are raw types (i32), not Message<i32>.
//! // Internally, data flows as Message<i32> through channels.
//! let connection: Connection<
//!     ProducerNode<VecProducer<i32>, (i32,)>,
//!     TransformerNode<MapTransformer<i32, String>, (i32,), (String,)>,
//!     0,  // Source port index (compile-time only)
//!     0,  // Target port index (compile-time only)
//! > = Connection::new();
//!
//! // This would fail to compile if:
//! // - Port indices are out of bounds
//! // - Output type (i32) doesn't match input type
//! // - Nodes don't have the required ports
//! ```

use super::nodes::{
  ConsumerNode, ProducerNode, TransformerNode, ValidateConsumerPorts, ValidateProducerPorts,
  ValidateTransformerPorts,
};
use crate::Consumer;
use crate::Producer;
use crate::Transformer;
use crate::port::{GetPort, PortList};

/// Trait for extracting an output port type from a node at a specific index.
///
/// This trait enables compile-time extraction of output port types from nodes,
/// which is used for connection type validation.
///
/// # Note
///
/// The port index `N` is used for compile-time type checking only. At runtime,
/// ports are identified by string names. The index corresponds to the position
/// in the `output_port_names()` vector.
///
/// Port types are raw types (e.g., `i32`), not `Message<T>`. Internally, nodes
/// wrap outputs in `Message<T>` before sending through channels, but the port
/// type validation uses raw types for a cleaner user-facing API.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::connection::HasOutputPort;
///
/// type OutputType = <ProducerNode<VecProducer<i32>, (i32,)> as HasOutputPort<0>>::OutputType;
/// // OutputType = i32 (raw type, not Message<i32>)
/// // Index 0 corresponds to the first port name in output_port_names()
/// // Internally, data flows as Message<i32> through channels
/// ```
pub trait HasOutputPort<const N: usize> {
  /// The type of the output port at index `N`.
  type OutputType;
}

/// Trait for extracting an input port type from a node at a specific index.
///
/// This trait enables compile-time extraction of input port types from nodes,
/// which is used for connection type validation.
///
/// # Note
///
/// The port index `N` is used for compile-time type checking only. At runtime,
/// ports are identified by string names. The index corresponds to the position
/// in the `input_port_names()` vector.
///
/// Port types are raw types (e.g., `i32`), not `Message<T>`. Internally, nodes
/// unwrap `Message<T>` from channels before processing, but the port type
/// validation uses raw types for a cleaner user-facing API.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::connection::HasInputPort;
///
/// type InputType = <ConsumerNode<VecConsumer<i32>, (i32,)> as HasInputPort<0>>::InputType;
/// // InputType = i32 (raw type, not Message<i32>)
/// // Index 0 corresponds to the first port name in input_port_names()
/// // Internally, data flows as Message<i32> through channels
/// ```
pub trait HasInputPort<const N: usize> {
  /// The type of the input port at index `N`.
  type InputType;
}

/// Trait for validating type compatibility between ports.
///
/// This trait ensures that an output port type is compatible with an input port type.
/// For now, this uses type equality, but could be extended to support conversions
/// or subtyping in the future.
///
/// # Note on Message<T>
///
/// Port types are validated using raw types (e.g., `i32`), not `Message<T>`. This
/// keeps the user-facing API clean. Internally, all data flows as `Message<T>`
/// through channels, but connection validation works with raw types.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::connection::CompatibleWith;
///
/// // i32 is compatible with i32
/// // Internally, Message<i32> flows through channels, but validation uses raw types
/// fn check_compatibility<T: CompatibleWith<i32>>(_: T) {}
/// ```
pub trait CompatibleWith<T> {}

// Implement CompatibleWith for type equality (most common case)
impl<T> CompatibleWith<T> for T {}

// Implement HasOutputPort for ProducerNode
impl<P, Outputs, const N: usize> HasOutputPort<N> for ProducerNode<P, Outputs>
where
  P: Producer,
  P::Output: std::fmt::Debug + Clone + Send + Sync,
  Outputs: PortList,
  Outputs: GetPort<N>,
  (): ValidateProducerPorts<P, Outputs>,
{
  type OutputType = <Outputs as GetPort<N>>::Type;
}

// Implement HasOutputPort for TransformerNode
impl<T, Inputs, Outputs, const N: usize> HasOutputPort<N> for TransformerNode<T, Inputs, Outputs>
where
  T: Transformer,
  T::Input: std::fmt::Debug + Clone + Send + Sync,
  T::Output: std::fmt::Debug + Clone + Send + Sync,
  Inputs: PortList,
  Outputs: PortList,
  Outputs: GetPort<N>,
  (): ValidateTransformerPorts<T, Inputs, Outputs>,
{
  type OutputType = <Outputs as GetPort<N>>::Type;
}

// Implement HasInputPort for TransformerNode
impl<T, Inputs, Outputs, const N: usize> HasInputPort<N> for TransformerNode<T, Inputs, Outputs>
where
  T: Transformer,
  T::Input: std::fmt::Debug + Clone + Send + Sync,
  T::Output: std::fmt::Debug + Clone + Send + Sync,
  Inputs: PortList,
  Outputs: PortList,
  Inputs: GetPort<N>,
  (): ValidateTransformerPorts<T, Inputs, Outputs>,
{
  type InputType = <Inputs as GetPort<N>>::Type;
}

// Implement HasInputPort for ConsumerNode
impl<C, Inputs, const N: usize> HasInputPort<N> for ConsumerNode<C, Inputs>
where
  C: Consumer,
  C::Input: std::fmt::Debug + Clone + Send + Sync,
  Inputs: PortList,
  Inputs: GetPort<N>,
  (): ValidateConsumerPorts<C, Inputs>,
{
  type InputType = <Inputs as GetPort<N>>::Type;
}

/// A connection between two nodes in the graph.
///
/// Connections are validated at compile time to ensure:
/// - Port indices exist on both nodes
/// - Output port type is compatible with input port type
/// - Nodes have the required ports
///
/// # Type Parameters
///
/// * `SourceNode` - The source node type (must implement `HasOutputPort<SOURCE_PORT>`)
/// * `TargetNode` - The target node type (must implement `HasInputPort<TARGET_PORT>`)
/// * `SOURCE_PORT` - The compile-time constant index of the source output port (for type checking only)
/// * `TARGET_PORT` - The compile-time constant index of the target input port (for type checking only)
///
/// # Note
///
/// The port indices (`SOURCE_PORT`, `TARGET_PORT`) are used for compile-time type validation only.
/// At runtime, ports are identified by string names. When using `GraphBuilder::connect()`, the
/// port indices are converted to port names when creating the runtime `ConnectionInfo`.
///
/// Port types are raw types (e.g., `i32`), not `Message<T>`. Internally, all data flowing
/// through the connection is wrapped in `Message<T>`, but the connection validation uses
/// raw types for a cleaner user-facing API.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::connection::Connection;
/// use streamweave::graph::node::{ProducerNode, TransformerNode};
/// use streamweave_VecProducer;
/// use streamweave_transformers::MapTransformer;
///
/// // Create a type-safe connection
/// // Port indices (0, 0) are for compile-time validation only.
/// // At runtime, these correspond to port names from output_port_names() and input_port_names().
/// // Port types are raw types (i32), but data flows as Message<i32> internally.
/// let connection: Connection<
///     ProducerNode<VecProducer<i32>, (i32,)>,
///     TransformerNode<MapTransformer<i32, i32>, (i32,), (i32,)>,
///     0,  // Compile-time port index (runtime uses port name from output_port_names()[0])
///     0,  // Compile-time port index (runtime uses port name from input_port_names()[0])
/// > = Connection::new();
/// ```
pub struct Connection<SourceNode, TargetNode, const SOURCE_PORT: usize, const TARGET_PORT: usize>
where
  SourceNode: HasOutputPort<SOURCE_PORT>,
  TargetNode: HasInputPort<TARGET_PORT>,
  <SourceNode as HasOutputPort<SOURCE_PORT>>::OutputType:
    CompatibleWith<<TargetNode as HasInputPort<TARGET_PORT>>::InputType>,
{
  // Connection metadata can be added here in the future
  // For now, the connection is purely type-level
  _phantom: std::marker::PhantomData<(SourceNode, TargetNode)>,
}

impl<SourceNode, TargetNode, const SOURCE_PORT: usize, const TARGET_PORT: usize>
  Connection<SourceNode, TargetNode, SOURCE_PORT, TARGET_PORT>
where
  SourceNode: HasOutputPort<SOURCE_PORT>,
  TargetNode: HasInputPort<TARGET_PORT>,
  <SourceNode as HasOutputPort<SOURCE_PORT>>::OutputType:
    CompatibleWith<<TargetNode as HasInputPort<TARGET_PORT>>::InputType>,
{
  /// Creates a new connection.
  ///
  /// The connection is validated at compile time through trait bounds.
  /// If the connection is invalid (wrong types, invalid ports), the code
  /// will fail to compile.
  ///
  /// # Returns
  ///
  /// A new `Connection` instance.
  pub fn new() -> Self {
    Self {
      _phantom: std::marker::PhantomData,
    }
  }

  /// Returns the source port index.
  ///
  /// # Returns
  ///
  /// The compile-time constant source port index.
  pub const fn source_port() -> usize {
    SOURCE_PORT
  }

  /// Returns the target port index.
  ///
  /// # Returns
  ///
  /// The compile-time constant target port index.
  pub const fn target_port() -> usize {
    TARGET_PORT
  }
}

impl<SourceNode, TargetNode, const SOURCE_PORT: usize, const TARGET_PORT: usize> Default
  for Connection<SourceNode, TargetNode, SOURCE_PORT, TARGET_PORT>
where
  SourceNode: HasOutputPort<SOURCE_PORT>,
  TargetNode: HasInputPort<TARGET_PORT>,
  <SourceNode as HasOutputPort<SOURCE_PORT>>::OutputType:
    CompatibleWith<<TargetNode as HasInputPort<TARGET_PORT>>::InputType>,
{
  fn default() -> Self {
    Self::new()
  }
}

/// Error type for connection-related errors.
///
/// Most connection validation happens at compile time, but this type exists
/// for potential runtime errors or future use cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionError {
  /// Invalid port name (shouldn't happen with const generics, but for safety)
  InvalidPortName {
    /// The invalid port name
    port_name: String,
    /// The available port names
    available_ports: Vec<String>,
  },
  /// Type mismatch (shouldn't happen with trait bounds, but for safety)
  TypeMismatch {
    /// Description of the type mismatch
    message: String,
  },
}

impl std::fmt::Display for ConnectionError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ConnectionError::InvalidPortName {
        port_name,
        available_ports,
      } => {
        write!(
          f,
          "Invalid port name: '{}' (available ports: {:?})",
          port_name, available_ports
        )
      }
      ConnectionError::TypeMismatch { message } => {
        write!(f, "Type mismatch: {}", message)
      }
    }
  }
}

impl std::error::Error for ConnectionError {}
