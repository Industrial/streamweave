//! # Pipeline DAG Representation
//!
//! This module defines the core data structures for representing a StreamWeave pipeline
//! as a Directed Acyclic Graph (DAG).
//!
//! ## Structure
//!
//! A pipeline DAG consists of:
//! - **Nodes**: Represent pipeline components (producers, transformers, consumers)
//! - **Edges**: Represent data flow between components
//! - **Metadata**: Additional information about each component (type, config, etc.)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the kind of a pipeline node.
///
/// Nodes can be producers (sources), transformers (processing), or consumers (sinks).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
  /// A producer node that generates data
  Producer,
  /// A transformer node that processes data
  Transformer,
  /// A consumer node that consumes data
  Consumer,
}

/// Metadata associated with a pipeline node.
///
/// Contains information about the component type, configuration, and other properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMetadata {
  /// The component type name (e.g., "ArrayProducer", "MapTransformer")
  pub component_type: String,
  /// The configured name of the component, if any
  pub name: Option<String>,
  /// Input type name for this component
  pub input_type: Option<String>,
  /// Output type name for this component
  pub output_type: Option<String>,
  /// Error strategy configuration
  pub error_strategy: String,
  /// Additional custom metadata
  #[serde(default)]
  pub custom: HashMap<String, String>,
}

impl Default for NodeMetadata {
  fn default() -> Self {
    Self {
      component_type: String::new(),
      name: None,
      input_type: None,
      output_type: None,
      error_strategy: "Stop".to_string(),
      custom: HashMap::new(),
    }
  }
}

/// Represents a node in the pipeline DAG.
///
/// Each node corresponds to a component in the pipeline (producer, transformer, or consumer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DagNode {
  /// Unique identifier for the node
  pub id: String,
  /// The kind of node (producer, transformer, consumer)
  pub kind: NodeKind,
  /// Metadata about the node
  pub metadata: NodeMetadata,
}

impl DagNode {
  /// Creates a new DAG node.
  ///
  /// # Arguments
  ///
  /// * `id` - Unique identifier for the node
  /// * `kind` - The kind of node
  /// * `metadata` - Metadata about the node
  ///
  /// # Returns
  ///
  /// A new `DagNode` instance.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::{DagNode, NodeKind, NodeMetadata};
  ///
  /// let metadata = NodeMetadata {
  ///     component_type: "ArrayProducer".to_string(),
  ///     name: Some("my_producer".to_string()),
  ///     input_type: None,
  ///     output_type: Some("i32".to_string()),
  ///     error_strategy: "Stop".to_string(),
  ///     custom: std::collections::HashMap::new(),
  /// };
  ///
  /// let node = DagNode::new("producer_1".to_string(), NodeKind::Producer, metadata);
  /// ```
  #[must_use]
  pub fn new(id: String, kind: NodeKind, metadata: NodeMetadata) -> Self {
    Self { id, kind, metadata }
  }
}

/// Represents an edge in the pipeline DAG.
///
/// Edges represent data flow from one component to another.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DagEdge {
  /// ID of the source node
  pub from: String,
  /// ID of the target node
  pub to: String,
  /// Optional label for the edge (e.g., data type)
  pub label: Option<String>,
}

impl DagEdge {
  /// Creates a new DAG edge.
  ///
  /// # Arguments
  ///
  /// * `from` - ID of the source node
  /// * `to` - ID of the target node
  /// * `label` - Optional label for the edge
  ///
  /// # Returns
  ///
  /// A new `DagEdge` instance.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::DagEdge;
  ///
  /// let edge = DagEdge::new(
  ///     "producer_1".to_string(),
  ///     "transformer_1".to_string(),
  ///     Some("i32".to_string()),
  /// );
  /// ```
  #[must_use]
  pub fn new(from: String, to: String, label: Option<String>) -> Self {
    Self { from, to, label }
  }
}

/// Represents a complete pipeline as a Directed Acyclic Graph (DAG).
///
/// This structure captures the entire pipeline structure including all nodes (components)
/// and edges (data flow connections) with their associated metadata.
///
/// # Example
///
/// ```rust
/// use streamweave::prelude::*;
/// use streamweave::visualization::PipelineDag;
///
/// let pipeline = Pipeline::new()
///     .with_producer(ArrayProducer::new(vec![1, 2, 3]))
///     .with_transformer(MapTransformer::new(|x| x * 2))
///     .with_consumer(VecConsumer::new());
///
/// let dag = PipelineDag::from_pipeline(&pipeline);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineDag {
  /// All nodes in the pipeline
  pub nodes: Vec<DagNode>,
  /// All edges connecting nodes
  pub edges: Vec<DagEdge>,
  /// Pipeline-level metadata
  #[serde(default)]
  pub metadata: HashMap<String, String>,
}

impl PipelineDag {
  /// Creates a new empty pipeline DAG.
  ///
  /// # Returns
  ///
  /// A new empty `PipelineDag` with no nodes or edges.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::PipelineDag;
  ///
  /// let dag = PipelineDag::new();
  /// assert!(dag.nodes.is_empty());
  /// assert!(dag.edges.is_empty());
  /// ```
  #[must_use]
  pub fn new() -> Self {
    Self {
      nodes: Vec::new(),
      edges: Vec::new(),
      metadata: HashMap::new(),
    }
  }

  /// Creates a pipeline DAG by extracting metadata from components.
  ///
  /// This method creates a DAG representation by querying component metadata
  /// from the provided component references. Use `DagBuilder` for a more convenient
  /// builder-pattern API.
  ///
  /// # Arguments
  ///
  /// * `producer` - Reference to the producer component
  /// * `transformer` - Reference to the transformer component
  /// * `consumer` - Reference to the consumer component
  ///
  /// # Returns
  ///
  /// A `PipelineDag` representing the pipeline structure.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::prelude::*;
  /// use streamweave::visualization::PipelineDag;
  ///
  /// let producer = ArrayProducer::new(vec![1, 2, 3]);
  /// let transformer = MapTransformer::new(|x| x * 2);
  /// let consumer = VecConsumer::new();
  ///
  /// let dag = PipelineDag::from_components(&producer, &transformer, &consumer);
  /// assert_eq!(dag.nodes.len(), 3); // producer, transformer, consumer
  /// assert_eq!(dag.edges.len(), 2); // producer->transformer, transformer->consumer
  /// ```
  #[must_use]
  pub fn from_components<P, T, C>(producer: &P, transformer: &T, consumer: &C) -> Self
  where
    P: crate::Producer,
    T: crate::Transformer,
    C: crate::Consumer,
  {
    let mut dag = Self::new();

    // Create producer node
    let producer_info = producer.component_info();
    let producer_config = producer.config();
    let producer_metadata = NodeMetadata {
      component_type: producer_info.type_name,
      name: producer_config.name(),
      input_type: None,
      output_type: Some(format_type_name(std::any::type_name::<P::Output>())),
      error_strategy: format_error_strategy(&producer_config.error_strategy()),
      custom: std::collections::HashMap::new(),
    };
    let producer_node = DagNode::new(
      "producer".to_string(),
      NodeKind::Producer,
      producer_metadata,
    );
    dag.add_node(producer_node.clone());

    // Create transformer node
    let transformer_info = transformer.component_info();
    let transformer_config = transformer.config();
    let transformer_metadata = NodeMetadata {
      component_type: transformer_info.type_name,
      name: transformer_config.name(),
      input_type: Some(format_type_name(std::any::type_name::<T::Input>())),
      output_type: Some(format_type_name(std::any::type_name::<T::Output>())),
      error_strategy: format_error_strategy(&transformer_config.error_strategy()),
      custom: std::collections::HashMap::new(),
    };
    let transformer_node = DagNode::new(
      "transformer".to_string(),
      NodeKind::Transformer,
      transformer_metadata,
    );
    dag.add_node(transformer_node.clone());

    // Create consumer node
    let consumer_info = consumer.component_info();
    let consumer_config = consumer.config();
    let consumer_metadata = NodeMetadata {
      component_type: consumer_info.type_name,
      name: Some(consumer_config.name.clone()),
      input_type: Some(format_type_name(std::any::type_name::<C::Input>())),
      output_type: None,
      error_strategy: format_error_strategy(&consumer_config.error_strategy),
      custom: std::collections::HashMap::new(),
    };
    let consumer_node = DagNode::new(
      "consumer".to_string(),
      NodeKind::Consumer,
      consumer_metadata,
    );
    dag.add_node(consumer_node);

    // Create edges
    dag.add_edge(DagEdge::new(
      "producer".to_string(),
      "transformer".to_string(),
      Some(format_type_name(std::any::type_name::<P::Output>())),
    ));
    dag.add_edge(DagEdge::new(
      "transformer".to_string(),
      "consumer".to_string(),
      Some(format_type_name(std::any::type_name::<T::Output>())),
    ));

    dag
  }

  /// Adds a node to the DAG.
  ///
  /// # Arguments
  ///
  /// * `node` - The node to add
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::{PipelineDag, DagNode, NodeKind, NodeMetadata};
  ///
  /// let mut dag = PipelineDag::new();
  /// let node = DagNode::new(
  ///     "node_1".to_string(),
  ///     NodeKind::Producer,
  ///     NodeMetadata::default(),
  /// );
  /// dag.add_node(node);
  /// ```
  pub fn add_node(&mut self, node: DagNode) {
    self.nodes.push(node);
  }

  /// Adds an edge to the DAG.
  ///
  /// # Arguments
  ///
  /// * `edge` - The edge to add
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::{PipelineDag, DagEdge};
  ///
  /// let mut dag = PipelineDag::new();
  /// let edge = DagEdge::new(
  ///     "node_1".to_string(),
  ///     "node_2".to_string(),
  ///     None,
  /// );
  /// dag.add_edge(edge);
  /// ```
  pub fn add_edge(&mut self, edge: DagEdge) {
    self.edges.push(edge);
  }

  /// Gets a reference to all nodes in the DAG.
  ///
  /// # Returns
  ///
  /// A slice of all nodes.
  #[must_use]
  pub fn nodes(&self) -> &[DagNode] {
    &self.nodes
  }

  /// Gets a reference to all edges in the DAG.
  ///
  /// # Returns
  ///
  /// A slice of all edges.
  #[must_use]
  pub fn edges(&self) -> &[DagEdge] {
    &self.edges
  }
}

impl Default for PipelineDag {
  fn default() -> Self {
    Self::new()
  }
}

/// Formats a type name for display in DAG metadata.
///
/// Extracts the short type name from a full Rust type path.
///
/// # Arguments
///
/// * `type_name` - The full type name from `std::any::type_name`
///
/// # Returns
///
/// A simplified type name for display.
fn format_type_name(type_name: &str) -> String {
  type_name
    .split("::")
    .last()
    .unwrap_or(type_name)
    .to_string()
}

/// Formats an error strategy for display in DAG metadata.
///
/// # Arguments
///
/// * `strategy` - The error strategy to format
///
/// # Returns
///
/// A string representation of the error strategy.
fn format_error_strategy<T>(strategy: &crate::error::ErrorStrategy<T>) -> String
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    crate::error::ErrorStrategy::Stop => "Stop".to_string(),
    crate::error::ErrorStrategy::Skip => "Skip".to_string(),
    crate::error::ErrorStrategy::Retry(n) => format!("Retry({})", n),
    crate::error::ErrorStrategy::Custom(_) => "Custom".to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_dag_node_creation() {
    let metadata = NodeMetadata {
      component_type: "TestProducer".to_string(),
      name: Some("test_producer".to_string()),
      input_type: None,
      output_type: Some("i32".to_string()),
      error_strategy: "Stop".to_string(),
      custom: HashMap::new(),
    };

    let node = DagNode::new("node_1".to_string(), NodeKind::Producer, metadata.clone());
    assert_eq!(node.id, "node_1");
    assert_eq!(node.kind, NodeKind::Producer);
    assert_eq!(node.metadata, metadata);
  }

  #[test]
  fn test_dag_edge_creation() {
    let edge = DagEdge::new(
      "node_1".to_string(),
      "node_2".to_string(),
      Some("i32".to_string()),
    );
    assert_eq!(edge.from, "node_1");
    assert_eq!(edge.to, "node_2");
    assert_eq!(edge.label, Some("i32".to_string()));
  }

  #[test]
  fn test_pipeline_dag_creation() {
    let dag = PipelineDag::new();
    assert!(dag.nodes.is_empty());
    assert!(dag.edges.is_empty());
  }

  #[test]
  fn test_pipeline_dag_add_node() {
    let mut dag = PipelineDag::new();
    let node = DagNode::new(
      "node_1".to_string(),
      NodeKind::Producer,
      NodeMetadata::default(),
    );
    dag.add_node(node);
    assert_eq!(dag.nodes().len(), 1);
  }

  #[test]
  fn test_pipeline_dag_add_edge() {
    let mut dag = PipelineDag::new();
    let edge = DagEdge::new("node_1".to_string(), "node_2".to_string(), None);
    dag.add_edge(edge);
    assert_eq!(dag.edges().len(), 1);
  }
}
