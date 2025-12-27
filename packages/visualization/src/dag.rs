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
    P: streamweave::Producer,
    T: streamweave::Transformer,
    C: streamweave::Consumer,
    P::Output: std::fmt::Debug + Clone + Send + Sync,
    T::Input: std::fmt::Debug + Clone + Send + Sync,
    T::Output: std::fmt::Debug + Clone + Send + Sync,
    C::Input: std::fmt::Debug + Clone + Send + Sync,
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
fn format_error_strategy<T>(strategy: &streamweave_error::ErrorStrategy<T>) -> String
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    streamweave_error::ErrorStrategy::Stop => "Stop".to_string(),
    streamweave_error::ErrorStrategy::Skip => "Skip".to_string(),
    streamweave_error::ErrorStrategy::Retry(n) => format!("Retry({})", n),
    streamweave_error::ErrorStrategy::Custom(_) => "Custom".to_string(),
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

  #[test]
  fn test_node_kind_variants() {
    assert_eq!(NodeKind::Producer, NodeKind::Producer);
    assert_eq!(NodeKind::Transformer, NodeKind::Transformer);
    assert_eq!(NodeKind::Consumer, NodeKind::Consumer);
    assert_ne!(NodeKind::Producer, NodeKind::Transformer);
    assert_ne!(NodeKind::Producer, NodeKind::Consumer);
    assert_ne!(NodeKind::Transformer, NodeKind::Consumer);
  }

  #[test]
  fn test_node_metadata_default() {
    let metadata = NodeMetadata::default();
    assert_eq!(metadata.component_type, "");
    assert_eq!(metadata.name, None);
    assert_eq!(metadata.input_type, None);
    assert_eq!(metadata.output_type, None);
    assert_eq!(metadata.error_strategy, "Stop");
    assert!(metadata.custom.is_empty());
  }

  #[test]
  fn test_node_metadata_with_custom() {
    let mut custom = HashMap::new();
    custom.insert("key1".to_string(), "value1".to_string());
    custom.insert("key2".to_string(), "value2".to_string());

    let metadata = NodeMetadata {
      component_type: "TestComponent".to_string(),
      name: Some("test".to_string()),
      input_type: Some("i32".to_string()),
      output_type: Some("String".to_string()),
      error_strategy: "Skip".to_string(),
      custom: custom.clone(),
    };

    assert_eq!(metadata.custom.len(), 2);
    assert_eq!(metadata.custom.get("key1"), Some(&"value1".to_string()));
    assert_eq!(metadata.custom.get("key2"), Some(&"value2".to_string()));
  }

  #[test]
  fn test_dag_edge_with_none_label() {
    let edge = DagEdge::new("node_1".to_string(), "node_2".to_string(), None);
    assert_eq!(edge.from, "node_1");
    assert_eq!(edge.to, "node_2");
    assert_eq!(edge.label, None);
  }

  #[test]
  fn test_pipeline_dag_default() {
    let dag = PipelineDag::default();
    assert!(dag.nodes.is_empty());
    assert!(dag.edges.is_empty());
    assert!(dag.metadata.is_empty());
  }

  #[test]
  fn test_pipeline_dag_nodes() {
    let mut dag = PipelineDag::new();
    let node1 = DagNode::new(
      "node_1".to_string(),
      NodeKind::Producer,
      NodeMetadata::default(),
    );
    let node2 = DagNode::new(
      "node_2".to_string(),
      NodeKind::Transformer,
      NodeMetadata::default(),
    );
    dag.add_node(node1);
    dag.add_node(node2);

    let nodes = dag.nodes();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].id, "node_1");
    assert_eq!(nodes[1].id, "node_2");
  }

  #[test]
  fn test_pipeline_dag_edges() {
    let mut dag = PipelineDag::new();
    let edge1 = DagEdge::new("node_1".to_string(), "node_2".to_string(), None);
    let edge2 = DagEdge::new(
      "node_2".to_string(),
      "node_3".to_string(),
      Some("i32".to_string()),
    );
    dag.add_edge(edge1);
    dag.add_edge(edge2);

    let edges = dag.edges();
    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].from, "node_1");
    assert_eq!(edges[1].label, Some("i32".to_string()));
  }

  #[test]
  fn test_format_type_name() {
    assert_eq!(format_type_name("i32"), "i32");
    assert_eq!(format_type_name("std::vec::Vec<i32>"), "Vec<i32>");
    assert_eq!(
      format_type_name("streamweave::producers::ArrayProducer"),
      "ArrayProducer"
    );
    assert_eq!(format_type_name(""), "");
  }

  #[test]
  fn test_format_error_strategy() {
    use streamweave_error::ErrorStrategy;

    assert_eq!(format_error_strategy(&ErrorStrategy::<i32>::Stop), "Stop");
    assert_eq!(format_error_strategy(&ErrorStrategy::<i32>::Skip), "Skip");
    assert_eq!(
      format_error_strategy(&ErrorStrategy::<i32>::Retry(5)),
      "Retry(5)"
    );
    assert_eq!(
      format_error_strategy(&ErrorStrategy::<i32>::new_custom(|_| {
        streamweave_error::ErrorAction::Skip
      })),
      "Custom"
    );
  }

  #[test]
  fn test_dag_node_transformer() {
    let metadata = NodeMetadata {
      component_type: "MapTransformer".to_string(),
      name: Some("mapper".to_string()),
      input_type: Some("i32".to_string()),
      output_type: Some("i32".to_string()),
      error_strategy: "Retry(3)".to_string(),
      custom: HashMap::new(),
    };

    let node = DagNode::new("transformer_1".to_string(), NodeKind::Transformer, metadata);
    assert_eq!(node.kind, NodeKind::Transformer);
  }

  #[test]
  fn test_dag_node_consumer() {
    let metadata = NodeMetadata {
      component_type: "VecConsumer".to_string(),
      name: Some("collector".to_string()),
      input_type: Some("i32".to_string()),
      output_type: None,
      error_strategy: "Skip".to_string(),
      custom: HashMap::new(),
    };

    let node = DagNode::new("consumer_1".to_string(), NodeKind::Consumer, metadata);
    assert_eq!(node.kind, NodeKind::Consumer);
  }

  #[test]
  fn test_pipeline_dag_from_components() {
    use streamweave_consumer_vec::VecConsumer;
    use streamweave_producer_vec::VecProducer;
    use streamweave_transformer_map::MapTransformer;

    let producer = VecProducer::new(vec![1, 2, 3]);
    let transformer = MapTransformer::new(|x: i32| x * 2);
    let consumer = VecConsumer::<i32>::new();

    let dag = PipelineDag::from_components(&producer, &transformer, &consumer);

    assert_eq!(dag.nodes().len(), 3);
    assert_eq!(dag.edges().len(), 2);

    // Check producer node
    let producer_node = dag.nodes().iter().find(|n| n.id == "producer").unwrap();
    assert_eq!(producer_node.kind, NodeKind::Producer);
    assert!(producer_node.metadata.output_type.is_some());
    assert_eq!(producer_node.metadata.input_type, None);

    // Check transformer node
    let transformer_node = dag.nodes().iter().find(|n| n.id == "transformer").unwrap();
    assert_eq!(transformer_node.kind, NodeKind::Transformer);
    assert!(transformer_node.metadata.input_type.is_some());
    assert!(transformer_node.metadata.output_type.is_some());

    // Check consumer node
    let consumer_node = dag.nodes().iter().find(|n| n.id == "consumer").unwrap();
    assert_eq!(consumer_node.kind, NodeKind::Consumer);
    assert!(consumer_node.metadata.input_type.is_some());
    assert_eq!(consumer_node.metadata.output_type, None);

    // Check edges
    let edge1 = dag
      .edges()
      .iter()
      .find(|e| e.from == "producer" && e.to == "transformer")
      .unwrap();
    assert!(edge1.label.is_some());

    let edge2 = dag
      .edges()
      .iter()
      .find(|e| e.from == "transformer" && e.to == "consumer")
      .unwrap();
    assert!(edge2.label.is_some());
  }

  #[test]
  fn test_pipeline_dag_from_components_with_names() {
    use streamweave_consumer_vec::VecConsumer;
    use streamweave_producer_vec::VecProducer;
    use streamweave_transformer_map::MapTransformer;

    let producer = VecProducer::new(vec![1, 2, 3]).with_name("my_producer".to_string());
    let transformer = MapTransformer::new(|x: i32| x * 2).with_name("my_transformer".to_string());
    let consumer = VecConsumer::<i32>::new().with_name("my_consumer".to_string());

    let dag = PipelineDag::from_components(&producer, &transformer, &consumer);

    let producer_node = dag.nodes().iter().find(|n| n.id == "producer").unwrap();
    assert_eq!(producer_node.metadata.name, Some("my_producer".to_string()));

    let transformer_node = dag.nodes().iter().find(|n| n.id == "transformer").unwrap();
    assert_eq!(
      transformer_node.metadata.name,
      Some("my_transformer".to_string())
    );

    let consumer_node = dag.nodes().iter().find(|n| n.id == "consumer").unwrap();
    assert_eq!(consumer_node.metadata.name, Some("my_consumer".to_string()));
  }

  #[test]
  fn test_pipeline_dag_from_components_with_error_strategies() {
    use streamweave_consumer_vec::VecConsumer;
    use streamweave_error::ErrorStrategy;
    use streamweave_producer_vec::VecProducer;
    use streamweave_transformer_map::MapTransformer;

    let producer = VecProducer::new(vec![1, 2, 3]).with_error_strategy(ErrorStrategy::<i32>::Skip);
    let transformer =
      MapTransformer::new(|x: i32| x * 2).with_error_strategy(ErrorStrategy::<i32>::Retry(3));
    let consumer = VecConsumer::<i32>::new().with_error_strategy(ErrorStrategy::<i32>::Stop);

    let dag = PipelineDag::from_components(&producer, &transformer, &consumer);

    let producer_node = dag.nodes().iter().find(|n| n.id == "producer").unwrap();
    assert_eq!(producer_node.metadata.error_strategy, "Skip");

    let transformer_node = dag.nodes().iter().find(|n| n.id == "transformer").unwrap();
    assert_eq!(transformer_node.metadata.error_strategy, "Retry(3)");

    let consumer_node = dag.nodes().iter().find(|n| n.id == "consumer").unwrap();
    assert_eq!(consumer_node.metadata.error_strategy, "Stop");
  }

  #[test]
  fn test_pipeline_dag_multiple_nodes_and_edges() {
    let mut dag = PipelineDag::new();

    // Add multiple nodes
    dag.add_node(DagNode::new(
      "producer".to_string(),
      NodeKind::Producer,
      NodeMetadata::default(),
    ));
    dag.add_node(DagNode::new(
      "transformer1".to_string(),
      NodeKind::Transformer,
      NodeMetadata::default(),
    ));
    dag.add_node(DagNode::new(
      "transformer2".to_string(),
      NodeKind::Transformer,
      NodeMetadata::default(),
    ));
    dag.add_node(DagNode::new(
      "consumer".to_string(),
      NodeKind::Consumer,
      NodeMetadata::default(),
    ));

    // Add multiple edges
    dag.add_edge(DagEdge::new(
      "producer".to_string(),
      "transformer1".to_string(),
      None,
    ));
    dag.add_edge(DagEdge::new(
      "transformer1".to_string(),
      "transformer2".to_string(),
      Some("i32".to_string()),
    ));
    dag.add_edge(DagEdge::new(
      "transformer2".to_string(),
      "consumer".to_string(),
      None,
    ));

    assert_eq!(dag.nodes().len(), 4);
    assert_eq!(dag.edges().len(), 3);
  }

  #[test]
  fn test_node_metadata_clone() {
    let mut custom = HashMap::new();
    custom.insert("test".to_string(), "value".to_string());

    let metadata1 = NodeMetadata {
      component_type: "Test".to_string(),
      name: Some("test".to_string()),
      input_type: Some("i32".to_string()),
      output_type: Some("String".to_string()),
      error_strategy: "Stop".to_string(),
      custom: custom.clone(),
    };

    let metadata2 = metadata1.clone();
    assert_eq!(metadata1, metadata2);
  }

  #[test]
  fn test_dag_edge_clone() {
    let edge1 = DagEdge::new(
      "node_1".to_string(),
      "node_2".to_string(),
      Some("i32".to_string()),
    );
    let edge2 = edge1.clone();
    assert_eq!(edge1, edge2);
  }

  #[test]
  fn test_dag_node_clone() {
    let node1 = DagNode::new(
      "node_1".to_string(),
      NodeKind::Producer,
      NodeMetadata::default(),
    );
    let node2 = node1.clone();
    assert_eq!(node1, node2);
  }
}
