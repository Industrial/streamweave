//! # Connection Types
//!
//! This module provides connection types that validate port compatibility at compile time
//! using trait bounds and const generics. Connections reference nodes by type and ports
//! by compile-time indices, ensuring type safety and port existence validation.
//!
//! ## Example
//!
//! ```rust
//! use streamweave::graph::connection::Connection;
//! use streamweave::graph::node::{ProducerNode, TransformerNode};
//! use streamweave::producers::vec::VecProducer;
//! use streamweave::transformers::map::MapTransformer;
//!
//! // Type-safe connection validated at compile time
//! let connection: Connection<
//!     ProducerNode<VecProducer<i32>, (i32,)>,
//!     TransformerNode<MapTransformer<i32, String>, (i32,), (String,)>,
//!     0,  // Source port index
//!     0,  // Target port index
//! > = Connection::new();
//!
//! // This would fail to compile if:
//! // - Port indices are out of bounds
//! // - Output type (i32) doesn't match input type
//! // - Nodes don't have the required ports
//! ```

use crate::consumer::Consumer;
use crate::graph::node::{
  ConsumerNode, ProducerNode, TransformerNode, ValidateConsumerPorts, ValidateProducerPorts,
  ValidateTransformerPorts,
};
use crate::graph::port::{GetPort, PortList};
use crate::producer::Producer;
use crate::transformer::Transformer;

/// Trait for extracting an output port type from a node at a specific index.
///
/// This trait enables compile-time extraction of output port types from nodes,
/// which is used for connection type validation.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::connection::HasOutputPort;
///
/// type OutputType = <ProducerNode<VecProducer<i32>, (i32,)> as HasOutputPort<0>>::OutputType;
/// // OutputType = i32
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
/// # Example
///
/// ```rust
/// use streamweave::graph::connection::HasInputPort;
///
/// type InputType = <ConsumerNode<VecConsumer<i32>, (i32,)> as HasInputPort<0>>::InputType;
/// // InputType = i32
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
/// # Example
///
/// ```rust
/// use streamweave::graph::connection::CompatibleWith;
///
/// // i32 is compatible with i32
/// fn check_compatibility<T: CompatibleWith<i32>>(_: T) {}
/// ```
pub trait CompatibleWith<T> {}

// Implement CompatibleWith for type equality (most common case)
impl<T> CompatibleWith<T> for T {}

// Implement HasOutputPort for ProducerNode
impl<P, Outputs, const N: usize> HasOutputPort<N> for ProducerNode<P, Outputs>
where
  P: Producer,
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
/// * `SOURCE_PORT` - The compile-time constant index of the source output port
/// * `TARGET_PORT` - The compile-time constant index of the target input port
///
/// # Example
///
/// ```rust
/// use streamweave::graph::connection::Connection;
/// use streamweave::graph::node::{ProducerNode, TransformerNode};
/// use streamweave::producers::vec::VecProducer;
/// use streamweave::transformers::map::MapTransformer;
///
/// // Create a type-safe connection
/// let connection: Connection<
///     ProducerNode<VecProducer<i32>, (i32,)>,
///     TransformerNode<MapTransformer<i32, i32>, (i32,), (i32,)>,
///     0,
///     0,
/// > = Connection::new();
/// ```
pub struct Connection<
  SourceNode,
  TargetNode,
  const SOURCE_PORT: usize,
  const TARGET_PORT: usize,
> where
  SourceNode: HasOutputPort<SOURCE_PORT>,
  TargetNode: HasInputPort<TARGET_PORT>,
  <SourceNode as HasOutputPort<SOURCE_PORT>>::OutputType: CompatibleWith<
    <TargetNode as HasInputPort<TARGET_PORT>>::InputType,
  >,
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
  <SourceNode as HasOutputPort<SOURCE_PORT>>::OutputType: CompatibleWith<
    <TargetNode as HasInputPort<TARGET_PORT>>::InputType,
  >,
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

/// Error type for connection-related errors.
///
/// Most connection validation happens at compile time, but this type exists
/// for potential runtime errors or future use cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionError {
  /// Invalid port index (shouldn't happen with const generics, but for safety)
  InvalidPortIndex {
    /// The invalid port index
    index: usize,
    /// The maximum valid port index
    max_index: usize,
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
      ConnectionError::InvalidPortIndex { index, max_index } => {
        write!(
          f,
          "Invalid port index: {} (max valid index: {})",
          index, max_index
        )
      }
      ConnectionError::TypeMismatch { message } => {
        write!(f, "Type mismatch: {}", message)
      }
    }
  }
}

impl std::error::Error for ConnectionError {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::consumers::vec::VecConsumer;
  use crate::producers::vec::VecProducer;
  use crate::transformers::map::MapTransformer;

  #[test]
  fn test_producer_to_transformer_connection() {
    // Valid connection: Producer<i32> -> Transformer<i32, String>
    type Source = ProducerNode<VecProducer<i32>, (i32,)>;
    type Target = TransformerNode<MapTransformer<i32, String>, (i32,), (String,)>;

    let _connection: Connection<Source, Target, 0, 0> = Connection::new();
    assert_eq!(Connection::<Source, Target, 0, 0>::source_port(), 0);
    assert_eq!(Connection::<Source, Target, 0, 0>::target_port(), 0);
  }

  #[test]
  fn test_transformer_to_consumer_connection() {
    // Valid connection: Transformer<i32, String> -> Consumer<String>
    type Source = TransformerNode<MapTransformer<i32, String>, (i32,), (String,)>;
    type Target = ConsumerNode<VecConsumer<String>, (String,)>;

    let _connection: Connection<Source, Target, 0, 0> = Connection::new();
    assert_eq!(Connection::<Source, Target, 0, 0>::source_port(), 0);
    assert_eq!(Connection::<Source, Target, 0, 0>::target_port(), 0);
  }

  #[test]
  fn test_producer_to_consumer_connection() {
    // Valid connection: Producer<i32> -> Consumer<i32>
    type Source = ProducerNode<VecProducer<i32>, (i32,)>;
    type Target = ConsumerNode<VecConsumer<i32>, (i32,)>;

    let _connection: Connection<Source, Target, 0, 0> = Connection::new();
  }

  #[test]
  fn test_transformer_to_transformer_connection() {
    // Valid connection: Transformer<i32, String> -> Transformer<String, bool>
    type Source = TransformerNode<MapTransformer<i32, String>, (i32,), (String,)>;
    type Target = TransformerNode<MapTransformer<String, bool>, (String,), (bool,)>;

    let _connection: Connection<Source, Target, 0, 0> = Connection::new();
  }

  #[test]
  fn test_connection_port_accessors() {
    type Source = ProducerNode<VecProducer<i32>, (i32,)>;
    type Target = ConsumerNode<VecConsumer<i32>, (i32,)>;

    let connection: Connection<Source, Target, 0, 0> = Connection::new();
    assert_eq!(Connection::<Source, Target, 0, 0>::source_port(), 0);
    assert_eq!(Connection::<Source, Target, 0, 0>::target_port(), 0);
  }

  #[test]
  fn test_connection_error_display() {
    let error = ConnectionError::InvalidPortIndex {
      index: 5,
      max_index: 3,
    };
    assert_eq!(
      error.to_string(),
      "Invalid port index: 5 (max valid index: 3)"
    );

    let error = ConnectionError::TypeMismatch {
      message: "Expected i32, got String".to_string(),
    };
    assert_eq!(error.to_string(), "Type mismatch: Expected i32, got String");
  }

  #[test]
  fn test_has_output_port_producer() {
    type Node = ProducerNode<VecProducer<i32>, (i32,)>;
    type OutputType = <Node as HasOutputPort<0>>::OutputType;
    let _: OutputType = 42i32;
  }

  #[test]
  fn test_has_output_port_transformer() {
    type Node = TransformerNode<MapTransformer<i32, String>, (i32,), (String,)>;
    type OutputType = <Node as HasOutputPort<0>>::OutputType;
    let _: OutputType = "hello".to_string();
  }

  #[test]
  fn test_has_input_port_transformer() {
    type Node = TransformerNode<MapTransformer<i32, String>, (i32,), (String,)>;
    type InputType = <Node as HasInputPort<0>>::InputType;
    let _: InputType = 42i32;
  }

  #[test]
  fn test_has_input_port_consumer() {
    type Node = ConsumerNode<VecConsumer<String>, (String,)>;
    type InputType = <Node as HasInputPort<0>>::InputType;
    let _: InputType = "hello".to_string();
  }

  #[test]
  fn test_compatible_with() {
    // i32 is compatible with i32
    fn check_compatibility<T: CompatibleWith<i32>>(_: T) {}
    check_compatibility::<i32>(42);
  }
}

