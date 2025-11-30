//! # Graph Node Types
//!
//! This module provides node types that wrap existing Producer, Transformer, and
//! Consumer traits with explicit port tuple information. Nodes use generic type
//! parameters for compile-time specialization.
//!
//! ## Example
//!
//! ```rust
//! use streamweave::graph::node::{ProducerNode, TransformerNode, ConsumerNode};
//! use streamweave::producers::array::ArrayProducer;
//! use streamweave::transformers::map::MapTransformer;
//! use streamweave::consumers::vec::VecConsumer;
//!
//! // Producer with single output
//! let producer = ProducerNode::new(
//!     "source".to_string(),
//!     ArrayProducer::new(vec![1, 2, 3]),
//! );
//!
//! // Transformer with single input and output
//! let transformer = TransformerNode::new(
//!     "mapper".to_string(),
//!     MapTransformer::new(|x: i32| x * 2),
//! );
//!
//! // Consumer with single input
//! let consumer = ConsumerNode::new(
//!     "sink".to_string(),
//!     VecConsumer::new(),
//! );
//! ```

use crate::consumer::Consumer;
use crate::graph::port::{GetPort, PortList};
use crate::graph::traits::{NodeKind, NodeTrait};
use crate::producer::Producer;
use crate::transformer::Transformer;

/// Trait for validating that a ProducerNode's output ports match the producer's output type.
///
/// This trait ensures type safety by requiring that the Outputs port tuple matches
/// the producer's Output type. For single-port producers, this means `Outputs = (P::Output,)`.
pub trait ValidateProducerPorts<P, Outputs>
where
  P: Producer,
  Outputs: PortList,
{
}

/// Trait for validating that a TransformerNode's ports match the transformer's input/output types.
///
/// This trait ensures type safety by requiring that the Inputs and Outputs port tuples
/// match the transformer's Input and Output types respectively.
pub trait ValidateTransformerPorts<T, Inputs, Outputs>
where
  T: Transformer,
  Inputs: PortList,
  Outputs: PortList,
{
}

/// Trait for validating that a ConsumerNode's input ports match the consumer's input type.
///
/// This trait ensures type safety by requiring that the Inputs port tuple matches
/// the consumer's Input type. For single-port consumers, this means `Inputs = (C::Input,)`.
pub trait ValidateConsumerPorts<C, Inputs>
where
  C: Consumer,
  Inputs: PortList,
{
}

// Implement validation for producer: first output port must match P::Output
// For single-port case: Outputs = (P::Output,)
impl<P, Outputs> ValidateProducerPorts<P, Outputs> for ()
where
  P: Producer,
  Outputs: PortList,
  Outputs: GetPort<0, Type = P::Output>,
{
}

// Implement validation for transformer: first input/output ports must match T::Input/T::Output
// For single-port case: Inputs = (T::Input,), Outputs = (T::Output,)
impl<T, Inputs, Outputs> ValidateTransformerPorts<T, Inputs, Outputs> for ()
where
  T: Transformer,
  Inputs: PortList,
  Outputs: PortList,
  Inputs: GetPort<0, Type = T::Input>,
  Outputs: GetPort<0, Type = T::Output>,
{
}

// Implement validation for consumer: first input port must match C::Input
// For single-port case: Inputs = (C::Input,)
impl<C, Inputs> ValidateConsumerPorts<C, Inputs> for ()
where
  C: Consumer,
  Inputs: PortList,
  Inputs: GetPort<0, Type = C::Input>,
{
}

/// A node that wraps a Producer component.
///
/// ProducerNode represents a source node in the graph that generates data streams.
/// It has no input ports (Inputs = ()) and one or more output ports specified by
/// the `Outputs` type parameter.
///
/// # Type Parameters
///
/// * `P` - The producer type that implements `Producer`
/// * `Outputs` - A port tuple representing the output ports (e.g., `(i32,)` for single port)
///
/// # Example
///
/// ```rust
/// use streamweave::graph::node::ProducerNode;
/// use streamweave::producers::array::ArrayProducer;
///
/// let node = ProducerNode::new(
///     "source".to_string(),
///     ArrayProducer::new(vec![1, 2, 3]),
/// );
/// ```
pub struct ProducerNode<P, Outputs>
where
  P: Producer,
  Outputs: PortList,
  (): ValidateProducerPorts<P, Outputs>,
{
  name: String,
  producer: P,
  // Router deferred to Phase 2
}

impl<P, Outputs> ProducerNode<P, Outputs>
where
  P: Producer,
  Outputs: PortList,
  (): ValidateProducerPorts<P, Outputs>,
{
  /// Creates a new ProducerNode with the given name and producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `producer` - The producer component to wrap
  ///
  /// # Returns
  ///
  /// A new `ProducerNode` instance.
  pub fn new(name: String, producer: P) -> Self {
    Self { name, producer }
  }

  /// Sets the name of this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The new name for this node
  ///
  /// # Returns
  ///
  /// The node with the updated name.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.name = name;
    self
  }

  /// Returns the name of this node.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Returns a reference to the wrapped producer.
  pub fn producer(&self) -> &P {
    &self.producer
  }

  /// Returns a mutable reference to the wrapped producer.
  pub fn producer_mut(&mut self) -> &mut P {
    &mut self.producer
  }
}

/// A node that wraps a Transformer component.
///
/// TransformerNode represents a processing node in the graph that transforms data streams.
/// It has one or more input ports and one or more output ports specified by the
/// `Inputs` and `Outputs` type parameters.
///
/// # Type Parameters
///
/// * `T` - The transformer type that implements `Transformer`
/// * `Inputs` - A port tuple representing the input ports (e.g., `(i32,)` for single port)
/// * `Outputs` - A port tuple representing the output ports (e.g., `(String,)` for single port)
///
/// # Example
///
/// ```rust
/// use streamweave::graph::node::TransformerNode;
/// use streamweave::transformers::map::MapTransformer;
///
/// let node = TransformerNode::new(
///     "mapper".to_string(),
///     MapTransformer::new(|x: i32| x * 2),
/// );
/// ```
pub struct TransformerNode<T, Inputs, Outputs>
where
  T: Transformer,
  Inputs: PortList,
  Outputs: PortList,
  (): ValidateTransformerPorts<T, Inputs, Outputs>,
{
  name: String,
  transformer: T,
  // Routers deferred to Phase 2
}

impl<T, Inputs, Outputs> TransformerNode<T, Inputs, Outputs>
where
  T: Transformer,
  Inputs: PortList,
  Outputs: PortList,
  (): ValidateTransformerPorts<T, Inputs, Outputs>,
{
  /// Creates a new TransformerNode with the given name and transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `transformer` - The transformer component to wrap
  ///
  /// # Returns
  ///
  /// A new `TransformerNode` instance.
  pub fn new(name: String, transformer: T) -> Self {
    Self { name, transformer }
  }

  /// Sets the name of this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The new name for this node
  ///
  /// # Returns
  ///
  /// The node with the updated name.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.name = name;
    self
  }

  /// Returns the name of this node.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Returns a reference to the wrapped transformer.
  pub fn transformer(&self) -> &T {
    &self.transformer
  }

  /// Returns a mutable reference to the wrapped transformer.
  pub fn transformer_mut(&mut self) -> &mut T {
    &mut self.transformer
  }
}

/// A node that wraps a Consumer component.
///
/// ConsumerNode represents a sink node in the graph that consumes data streams.
/// It has one or more input ports specified by the `Inputs` type parameter and
/// no output ports (Outputs = ()).
///
/// # Type Parameters
///
/// * `C` - The consumer type that implements `Consumer`
/// * `Inputs` - A port tuple representing the input ports (e.g., `(String,)` for single port)
///
/// # Example
///
/// ```rust
/// use streamweave::graph::node::ConsumerNode;
/// use streamweave::consumers::vec::VecConsumer;
///
/// let node = ConsumerNode::new(
///     "sink".to_string(),
///     VecConsumer::new(),
/// );
/// ```
pub struct ConsumerNode<C, Inputs>
where
  C: Consumer,
  Inputs: PortList,
  (): ValidateConsumerPorts<C, Inputs>,
{
  name: String,
  consumer: C,
  // Router deferred to Phase 2
}

impl<C, Inputs> ConsumerNode<C, Inputs>
where
  C: Consumer,
  Inputs: PortList,
  (): ValidateConsumerPorts<C, Inputs>,
{
  /// Creates a new ConsumerNode with the given name and consumer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name of this node
  /// * `consumer` - The consumer component to wrap
  ///
  /// # Returns
  ///
  /// A new `ConsumerNode` instance.
  pub fn new(name: String, consumer: C) -> Self {
    Self { name, consumer }
  }

  /// Sets the name of this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The new name for this node
  ///
  /// # Returns
  ///
  /// The node with the updated name.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.name = name;
    self
  }

  /// Returns the name of this node.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Returns a reference to the wrapped consumer.
  pub fn consumer(&self) -> &C {
    &self.consumer
  }

  /// Returns a mutable reference to the wrapped consumer.
  pub fn consumer_mut(&mut self) -> &mut C {
    &mut self.consumer
  }
}

// Implement NodeTrait for ProducerNode
impl<P, Outputs> NodeTrait for ProducerNode<P, Outputs>
where
  P: Producer,
  Outputs: PortList,
  (): ValidateProducerPorts<P, Outputs>,
{
  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Producer
  }
}

// Implement NodeTrait for TransformerNode
impl<T, Inputs, Outputs> NodeTrait for TransformerNode<T, Inputs, Outputs>
where
  T: Transformer,
  Inputs: PortList,
  Outputs: PortList,
  (): ValidateTransformerPorts<T, Inputs, Outputs>,
{
  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Transformer
  }
}

// Implement NodeTrait for ConsumerNode
impl<C, Inputs> NodeTrait for ConsumerNode<C, Inputs>
where
  C: Consumer,
  Inputs: PortList,
  (): ValidateConsumerPorts<C, Inputs>,
{
  fn name(&self) -> &str {
    &self.name
  }

  fn node_kind(&self) -> NodeKind {
    NodeKind::Consumer
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::consumers::vec::VecConsumer;
  use crate::producers::vec::VecProducer;
  use crate::transformers::map::MapTransformer;

  #[test]
  fn test_producer_node_creation() {
    let producer = VecProducer::new(vec![1, 2, 3]);
    let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);
    assert_eq!(node.name(), "source");
  }

  #[test]
  fn test_producer_node_with_name() {
    let producer = VecProducer::new(vec![1, 2, 3]);
    let node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer)
      .with_name("new_source".to_string());
    assert_eq!(node.name(), "new_source");
  }

  #[test]
  fn test_transformer_node_creation() {
    let transformer = MapTransformer::new(|x: i32| x * 2);
    let node: TransformerNode<_, (i32,), (i32,)> = TransformerNode::new("mapper".to_string(), transformer);
    assert_eq!(node.name(), "mapper");
  }

  #[test]
  fn test_transformer_node_with_name() {
    let transformer = MapTransformer::new(|x: i32| x * 2);
    let node: TransformerNode<_, (i32,), (i32,)> = TransformerNode::new("mapper".to_string(), transformer)
      .with_name("new_mapper".to_string());
    assert_eq!(node.name(), "new_mapper");
  }

  #[test]
  fn test_consumer_node_creation() {
    let consumer = VecConsumer::new();
    let node: ConsumerNode<_, (i32,)> = ConsumerNode::new("sink".to_string(), consumer);
    assert_eq!(node.name(), "sink");
  }

  #[test]
  fn test_consumer_node_with_name() {
    let consumer = VecConsumer::new();
    let node: ConsumerNode<_, (i32,)> = ConsumerNode::new("sink".to_string(), consumer)
      .with_name("new_sink".to_string());
    assert_eq!(node.name(), "new_sink");
  }

  #[test]
  fn test_producer_node_accessors() {
    let producer = VecProducer::new(vec![1, 2, 3]);
    let mut node: ProducerNode<_, (i32,)> = ProducerNode::new("source".to_string(), producer);
    
    assert_eq!(node.name(), "source");
    let _producer_ref = node.producer();
    let _producer_mut = node.producer_mut();
  }

  #[test]
  fn test_transformer_node_accessors() {
    let transformer = MapTransformer::new(|x: i32| x * 2);
    let mut node: TransformerNode<_, (i32,), (i32,)> = TransformerNode::new("mapper".to_string(), transformer);
    
    assert_eq!(node.name(), "mapper");
    let _transformer_ref = node.transformer();
    let _transformer_mut = node.transformer_mut();
  }

  #[test]
  fn test_consumer_node_accessors() {
    let consumer = VecConsumer::new();
    let mut node: ConsumerNode<_, (i32,)> = ConsumerNode::new("sink".to_string(), consumer);
    
    assert_eq!(node.name(), "sink");
    let _consumer_ref = node.consumer();
    let _consumer_mut = node.consumer_mut();
  }
}

