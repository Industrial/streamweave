use std::collections::HashMap;
use streamweave_visualization::*;

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
  use streamweave_visualization::format_type_name;
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
  use streamweave_visualization::format_error_strategy;

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
  use streamweave_transformers::map::MapTransformer;
  use streamweave_vec::VecConsumer;
  use streamweave_vec::VecProducer;

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
  use streamweave_transformers::map::MapTransformer;
  use streamweave_vec::VecConsumer;
  use streamweave_vec::VecProducer;

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
  use streamweave_error::ErrorStrategy;
  use streamweave_transformers::map::MapTransformer;
  use streamweave_vec::VecConsumer;
  use streamweave_vec::VecProducer;

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
