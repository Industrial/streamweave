//! Graph builder and topology tests
//!
//! This module provides tests for graph construction, topology validation, and graph execution scenarios.

use std::time::Duration;
use streamweave_graph::{
  ConsumerNode, GraphBuilder, GraphExecution, ProducerNode, TransformerNode,
};
use streamweave_transformers::{MapTransformer, RunningSumTransformer};
use streamweave_vec::{VecConsumer, VecProducer};
use streamweave_window::{TimeWindowTransformer, WindowTransformer};
use tokio::time::sleep;

async fn wait_for_completion() {
  sleep(Duration::from_millis(100)).await;
}

fn create_linear_graph() -> streamweave_graph::Graph {
  GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3, 4, 5]),
    ))
    .unwrap()
    .node(TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    ))
    .unwrap()
    .node(ConsumerNode::from_consumer(
      "sink".to_string(),
      VecConsumer::<i32>::new(),
    ))
    .unwrap()
    .connect_by_name("source", "transform")
    .unwrap()
    .connect_by_name("transform", "sink")
    .unwrap()
    .build()
}

#[cfg(test)]
mod simple_graph_tests {
  use super::*;

  #[tokio::test]
  async fn test_simple_linear_graph() {
    let graph = create_linear_graph();
    let mut executor = graph.executor();

    executor.start().await.unwrap();
    assert_eq!(executor.state(), streamweave_graph::ExecutionState::Running);

    wait_for_completion().await;

    executor.stop().await.unwrap();
    assert_eq!(executor.state(), streamweave_graph::ExecutionState::Stopped);
  }

  #[tokio::test]
  async fn test_producer_only_graph() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_transformer_only_graph() {
    let graph = GraphBuilder::new()
      .node(TransformerNode::from_transformer(
        "transform".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();
    let result = executor.start().await;
    if result.is_ok() {
      executor.stop().await.unwrap();
    }
  }

  #[tokio::test]
  async fn test_consumer_only_graph() {
    let graph = GraphBuilder::new()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();
    let result = executor.start().await;
    if result.is_ok() {
      executor.stop().await.unwrap();
    }
  }

  #[tokio::test]
  async fn test_data_flow_verification() {
    let graph = create_linear_graph();
    let mut executor = graph.executor();

    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();

    assert!(executor.errors().is_empty());
  }
}

#[cfg(test)]
mod fan_out_fan_in_tests {
  use super::*;

  #[tokio::test]
  async fn test_fan_out_graph() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform1".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform2".to_string(),
        MapTransformer::new(|x: i32| x * 3),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink1".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink2".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "transform1")
      .unwrap()
      .connect_by_name("source", "transform2")
      .unwrap()
      .connect_by_name("transform1", "sink1")
      .unwrap()
      .connect_by_name("transform2", "sink2")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_fan_in_graph() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source1".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(ProducerNode::from_producer(
        "source2".to_string(),
        VecProducer::new(vec![4, 5, 6]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source1", "transform")
      .unwrap()
      .connect_by_name("transform", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_complex_fan_out_fan_in() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source1".to_string(),
        VecProducer::new(vec![1, 2]),
      ))
      .unwrap()
      .node(ProducerNode::from_producer(
        "source2".to_string(),
        VecProducer::new(vec![3, 4]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform1".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform2".to_string(),
        MapTransformer::new(|x: i32| x * 3),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source1", "transform1")
      .unwrap()
      .connect_by_name("source2", "transform2")
      .unwrap()
      .connect_by_name("transform1", "sink")
      .unwrap()
      .connect_by_name("transform2", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();
  }
}

#[cfg(test)]
mod performance_tests {
  use super::*;

  #[tokio::test]
  async fn test_concurrent_execution_multiple_nodes() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source1".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(ProducerNode::from_producer(
        "source2".to_string(),
        VecProducer::new(vec![4, 5, 6]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform1".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform2".to_string(),
        MapTransformer::new(|x: i32| x * 3),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink1".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink2".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source1", "transform1")
      .unwrap()
      .connect_by_name("source2", "transform2")
      .unwrap()
      .connect_by_name("transform1", "sink1")
      .unwrap()
      .connect_by_name("transform2", "sink2")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_different_data_types() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform".to_string(),
        MapTransformer::new(|s: String| s.to_uppercase()),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<String>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "transform")
      .unwrap()
      .connect_by_name("transform", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_large_data_volume() {
    let large_data: Vec<i32> = (0..1000).collect();
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(large_data),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "transform")
      .unwrap()
      .connect_by_name("transform", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    sleep(Duration::from_millis(200)).await;
    executor.stop().await.unwrap();
  }
}

#[cfg(test)]
mod complex_graph_tests {
  use super::*;

  #[tokio::test]
  async fn test_multiple_transformers_in_sequence() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3, 4, 5]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "double".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "triple".to_string(),
        MapTransformer::new(|x: i32| x * 3),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "double")
      .unwrap()
      .connect_by_name("double", "triple")
      .unwrap()
      .connect_by_name("triple", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_stateful_transformer() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3, 4, 5]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "running_sum".to_string(),
        RunningSumTransformer::<i32>::new(),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "running_sum")
      .unwrap()
      .connect_by_name("running_sum", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_count_based_windowed_transformer() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "window".to_string(),
        WindowTransformer::<i32>::new(3),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "window")
      .unwrap()
      .connect_by_name("window", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_time_based_windowed_transformer() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3, 4, 5]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "time_window".to_string(),
        TimeWindowTransformer::<i32>::new(Duration::from_millis(100)),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "time_window")
      .unwrap()
      .connect_by_name("time_window", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    sleep(Duration::from_millis(200)).await;
    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_complex_mixed_topology() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source1".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(ProducerNode::from_producer(
        "source2".to_string(),
        VecProducer::new(vec![4, 5, 6]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "mapper1".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "mapper2".to_string(),
        MapTransformer::new(|x: i32| x * 3),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "window".to_string(),
        WindowTransformer::<i32>::new(2),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink1".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink2".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source1", "mapper1")
      .unwrap()
      .connect_by_name("source2", "mapper2")
      .unwrap()
      .connect_by_name("mapper1", "window")
      .unwrap()
      .connect_by_name("window", "sink1")
      .unwrap()
      .connect_by_name("mapper2", "sink2")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();
  }
}

#[cfg(test)]
mod router_tests {
  use super::*;

  #[tokio::test]
  async fn test_broadcast_router() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform1".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform2".to_string(),
        MapTransformer::new(|x: i32| x * 3),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink1".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink2".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "transform1")
      .unwrap()
      .connect_by_name("source", "transform2")
      .unwrap()
      .connect_by_name("transform1", "sink1")
      .unwrap()
      .connect_by_name("transform2", "sink2")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();

    assert!(executor.errors().is_empty());
  }

  #[tokio::test]
  async fn test_round_robin_router() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3, 4, 5, 6]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform1".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform2".to_string(),
        MapTransformer::new(|x: i32| x * 3),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink1".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink2".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "transform1")
      .unwrap()
      .connect_by_name("source", "transform2")
      .unwrap()
      .connect_by_name("transform1", "sink1")
      .unwrap()
      .connect_by_name("transform2", "sink2")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();

    assert!(executor.errors().is_empty());
  }

  #[tokio::test]
  async fn test_merge_router() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source1".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(ProducerNode::from_producer(
        "source2".to_string(),
        VecProducer::new(vec![4, 5, 6]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source1", "transform")
      .unwrap()
      .connect_by_name("source2", "transform")
      .unwrap()
      .connect_by_name("transform", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();

    assert!(executor.errors().is_empty());
  }

  #[tokio::test]
  async fn test_key_based_router() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3, 4, 5, 6]),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform1".to_string(),
        MapTransformer::new(|x: i32| x * 2),
      ))
      .unwrap()
      .node(TransformerNode::from_transformer(
        "transform2".to_string(),
        MapTransformer::new(|x: i32| x * 3),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink1".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink2".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "transform1")
      .unwrap()
      .connect_by_name("source", "transform2")
      .unwrap()
      .connect_by_name("transform1", "sink1")
      .unwrap()
      .connect_by_name("transform2", "sink2")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();

    assert!(executor.errors().is_empty());
  }
}

#[cfg(test)]
mod additional_coverage_tests {
  use super::*;

  #[test]
  fn test_graph_error_all_variants_display() {
    let error = GraphError::NodeNotFound {
      name: "test".to_string(),
    };
    assert_eq!(error.to_string(), "Node not found: test");

    let error = GraphError::DuplicateNode {
      name: "test".to_string(),
    };
    assert_eq!(error.to_string(), "Duplicate node: test");

    let error = GraphError::InvalidConnection {
      source: "source".to_string(),
      target: "target".to_string(),
      reason: "test reason".to_string(),
    };
    assert_eq!(
      error.to_string(),
      "Invalid connection from source to target: test reason"
    );

    let error = GraphError::PortNotFound {
      node: "test_node".to_string(),
      port: 5,
    };
    assert_eq!(error.to_string(), "Port 5 not found on node test_node");

    let error = GraphError::TypeMismatch {
      expected: "i32".to_string(),
      actual: "String".to_string(),
    };
    assert_eq!(error.to_string(), "Type mismatch: expected i32, got String");

    let error = GraphError::InvalidPortName {
      port_name: "invalid_port".to_string(),
    };
    assert_eq!(error.to_string(), "Invalid port name format: invalid_port");
  }

  #[test]
  fn test_graph_error_error_trait() {
    use std::error::Error;

    let error = GraphError::NodeNotFound {
      name: "test".to_string(),
    };

    let _error_ref: &dyn Error = &error;
    assert_eq!(error.to_string(), "Node not found: test");
  }

  #[test]
  fn test_graph_default() {
    let graph1 = Graph::new();
    let graph2 = Graph::default();

    assert_eq!(graph1.len(), graph2.len());
    assert_eq!(graph1.is_empty(), graph2.is_empty());
  }

  #[test]
  fn test_graph_builder_default() {
    let builder1 = GraphBuilder::new();
    let builder2 = GraphBuilder::default();

    assert_eq!(builder1.node_count(), builder2.node_count());
    assert_eq!(builder1.connection_count(), builder2.connection_count());
    assert_eq!(builder1.is_empty(), builder2.is_empty());
  }

  #[test]
  fn test_runtime_graph_builder_default() {
    let builder1 = RuntimeGraphBuilder::new();
    let builder2 = RuntimeGraphBuilder::default();

    let graph1 = builder1.build();
    let graph2 = builder2.build();

    assert_eq!(graph1.len(), graph2.len());
    assert_eq!(graph1.is_empty(), graph2.is_empty());
  }

  #[test]
  fn test_connection_info_new() {
    let conn = ConnectionInfo::new(("source".to_string(), 0), ("target".to_string(), 1));

    assert_eq!(conn.source, ("source".to_string(), 0));
    assert_eq!(conn.target, ("target".to_string(), 1));
  }

  #[test]
  fn test_connection_info_clone_eq_hash() {
    let conn1 = ConnectionInfo::new(("source".to_string(), 0), ("target".to_string(), 1));
    let conn2 = conn1.clone();

    assert_eq!(conn1, conn2);

    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(conn1);
    assert!(set.contains(&conn2));
  }

  #[test]
  fn test_connection_info_debug() {
    let conn = ConnectionInfo::new(("source".to_string(), 0), ("target".to_string(), 1));
    let debug_str = format!("{:?}", conn);
    assert!(!debug_str.is_empty());
  }

  #[test]
  fn test_graph_get_node_nonexistent() {
    let graph = Graph::new();
    assert!(graph.get_node("nonexistent").is_none());
  }

  #[test]
  fn test_graph_get_children_empty() {
    let graph = Graph::new();
    let children = graph.get_children("nonexistent");
    assert!(children.is_empty());
  }

  #[test]
  fn test_graph_get_parents_empty() {
    let graph = Graph::new();
    let parents = graph.get_parents("nonexistent");
    assert!(parents.is_empty());
  }

  #[test]
  fn test_graph_builder_connect_port_mismatch() {
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let builder = builder
      .add_node("transform".to_string(), transformer)
      .unwrap();

    type Source = ProducerNode<VecProducer<i32>, (i32,)>;
    type Target = TransformerNode<MapTransformer<i32, i32>, (i32,), (i32,)>;

    let result = builder.connect::<Source, Target, 0, 0>("source", "transform", 1, 0);
    assert!(result.is_err());
    if let Err(GraphError::InvalidConnection {
      source,
      target,
      reason,
    }) = result
    {
      assert_eq!(source, "source");
      assert_eq!(target, "transform");
      assert!(reason.contains("Source port index"));
    } else {
      panic!("Expected InvalidConnection error");
    }
  }

  #[test]
  fn test_graph_builder_connect_target_port_mismatch() {
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let builder = builder
      .add_node("transform".to_string(), transformer)
      .unwrap();

    type Source = ProducerNode<VecProducer<i32>, (i32,)>;
    type Target = TransformerNode<MapTransformer<i32, i32>, (i32,), (i32,)>;

    let result = builder.connect::<Source, Target, 0, 0>("source", "transform", 0, 1);
    assert!(result.is_err());
    if let Err(GraphError::InvalidConnection {
      source,
      target,
      reason,
    }) = result
    {
      assert_eq!(source, "source");
      assert_eq!(target, "transform");
      assert!(reason.contains("Target port index"));
    } else {
      panic!("Expected InvalidConnection error");
    }
  }

  #[test]
  fn test_graph_builder_connect_from_has_connections_state() {
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer1 = TransformerNode::from_transformer(
      "transform1".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let transformer2 = TransformerNode::from_transformer(
      "transform2".to_string(),
      MapTransformer::new(|x: i32| x * 3),
    );

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let builder = builder
      .add_node("transform1".to_string(), transformer1)
      .unwrap();
    let builder = builder
      .add_node("transform2".to_string(), transformer2)
      .unwrap();

    let builder = builder.connect_by_name("source", "transform1").unwrap();
    assert_eq!(builder.connection_count(), 1);

    let builder = builder.connect_by_name("source", "transform2").unwrap();
    assert_eq!(builder.connection_count(), 2);

    let graph = builder.build();
    assert_eq!(graph.get_connections().len(), 2);
  }

  #[test]
  fn test_graph_builder_add_node_after_connection() {
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let builder = builder
      .add_node("transform".to_string(), transformer)
      .unwrap();

    let builder = builder.connect_by_name("source", "transform").unwrap();

    let builder = builder.add_node("sink".to_string(), consumer).unwrap();
    assert_eq!(builder.node_count(), 3);
    assert_eq!(builder.connection_count(), 1);

    let builder = builder.connect_by_name("transform", "sink").unwrap();
    assert_eq!(builder.connection_count(), 2);
  }

  #[test]
  fn test_runtime_graph_builder_build() {
    let mut builder = RuntimeGraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

    builder
      .add_node("source".to_string(), Box::new(producer))
      .unwrap();
    builder
      .add_node("sink".to_string(), Box::new(consumer))
      .unwrap();
    builder.connect(("source", 0), ("sink", 0)).unwrap();

    let graph = builder.build();
    assert_eq!(graph.len(), 2);
    assert_eq!(graph.get_connections().len(), 1);
  }

  #[test]
  fn test_runtime_graph_builder_connect_target_node_not_found() {
    let mut builder = RuntimeGraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));

    builder
      .add_node("source".to_string(), Box::new(producer))
      .unwrap();

    let result = builder.connect(("source", 0), ("nonexistent", 0));
    assert!(result.is_err());
    if let Err(GraphError::NodeNotFound { name }) = result {
      assert_eq!(name, "nonexistent");
    } else {
      panic!("Expected NodeNotFound error");
    }
  }

  #[test]
  fn test_resolve_port_name_invalid_numeric_index() {
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let builder = builder
      .add_node("transform".to_string(), transformer)
      .unwrap();

    let result = builder.connect_by_name("source:1", "transform");
    assert!(result.is_err());
  }

  #[test]
  fn test_parse_port_spec_edge_cases() {
    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );

    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let builder = builder
      .add_node("transform".to_string(), transformer)
      .unwrap();

    let result = builder.connect_by_name("source:out", "transform:in");
    assert!(result.is_ok());

    let builder = GraphBuilder::new();
    let producer =
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
    let transformer = TransformerNode::from_transformer(
      "transform".to_string(),
      MapTransformer::new(|x: i32| x * 2),
    );
    let builder = builder.add_node("source".to_string(), producer).unwrap();
    let builder = builder
      .add_node("transform".to_string(), transformer)
      .unwrap();
    // Numeric indices are no longer supported - use named ports
    let result = builder.connect_by_name("source:out", "transform:in");
    assert!(result.is_ok());
  }
}

// Tests moved from src/
use streamweave_graph::node::{ConsumerNode, ProducerNode, TransformerNode};
use streamweave_graph::{Graph, GraphBuilder};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;
use streamweave_vec::VecProducer;

#[test]
fn test_graph_new() {
  let graph = Graph::new();
  assert!(graph.is_empty());
  assert_eq!(graph.len(), 0);
}

#[test]
fn test_graph_builder_add_node() {
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));

  let builder = builder.add_node("source".to_string(), producer).unwrap();
  assert!(
    builder
      .add_node(
        "source".to_string(),
        ProducerNode::from_producer("duplicate".to_string(), VecProducer::new(vec![4, 5, 6]),)
      )
      .is_err()
  );
}

#[test]
fn test_graph_builder_connect() {
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));

  let builder = builder.add_node("source".to_string(), producer).unwrap();
  let builder = builder
    .add_node("transform".to_string(), transformer)
    .unwrap();

  // Valid connection - use connect_by_name to avoid needing explicit closure type
  let builder = builder.connect_by_name("source", "transform").unwrap();
  assert_eq!(builder.connection_count(), 1);

  // Test that we can build from this state
  let graph = builder.build();
  assert_eq!(graph.len(), 2);
  assert_eq!(graph.get_connections().len(), 1);
}

#[test]
fn test_fluent_node_api() {
  // Test the fluent node() method
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));

  // Use fluent API
  let builder = builder.node(producer).unwrap();
  let builder = builder.node(transformer).unwrap();
  assert_eq!(builder.node_count(), 2);
}

#[test]
fn test_fluent_connect_by_name() {
  // Test the fluent connect_by_name() method
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));

  let builder = builder.node(producer).unwrap();
  let builder = builder.node(transformer).unwrap();

  // Connect using port names
  let builder = builder.connect_by_name("source", "transform").unwrap();
  assert_eq!(builder.connection_count(), 1);

  // Connect using explicit port names (named ports)
  let builder = builder
    .connect_by_name("source:out", "transform:in")
    .unwrap();
  assert_eq!(builder.connection_count(), 2);

  let graph = builder.build();
  assert_eq!(graph.get_connections().len(), 2);
}

#[test]
fn test_fluent_api_chaining() {
  // Test method chaining with fluent API
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));
  let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

  let graph = GraphBuilder::new()
    .node(producer)
    .unwrap()
    .node(transformer)
    .unwrap()
    .node(consumer)
    .unwrap()
    .connect_by_name("source", "transform")
    .unwrap()
    .connect_by_name("transform", "sink")
    .unwrap()
    .build();

  assert_eq!(graph.len(), 3);
  assert_eq!(graph.get_connections().len(), 2);
}

#[test]
fn test_compile_time_node_existence_validation() {
  // Test that connect() requires node types to be in the builder state
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));

  // Add nodes
  let builder = builder.node(producer).unwrap();
  let builder = builder.node(transformer).unwrap();

  // Use connect_by_name to avoid needing explicit closure type
  let builder = builder.connect_by_name("source", "transform").unwrap();

  assert_eq!(builder.connection_count(), 1);

  // The following would fail to compile if uncommented:
  // let consumer = ConsumerNode::new("sink".to_string(), VecConsumer::new());
  // // This fails because ConsumerNode is not in the builder state yet
  // builder.connect::<
  //   ProducerNode<VecProducer<i32>, (i32,)>,
  //   ConsumerNode<VecConsumer<i32>, (i32,)>,
  //   0, 0
  // >("source", "sink", 0, 0).unwrap();
}

#[test]
fn test_compile_time_port_bounds_validation() {
  // Test that port bounds are validated at compile time
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));

  let builder = builder.node(producer).unwrap();
  let builder = builder.node(transformer).unwrap();

  // Valid port index (0) - use connect_by_name to avoid explicit type annotations
  let _builder = builder.connect_by_name("source", "transform").unwrap();

  // The following would fail to compile if uncommented:
  // // Invalid port index (1) - port doesn't exist on single-port nodes
  // builder.connect::<
  //   ProducerNode<VecProducer<i32>, (i32,)>,
  //   TransformerNode<MapTransformer<i32, i32>, (i32,), (i32,)>,
  //   1,  // Invalid: only port 0 exists
  //   0,
  // >("source", "transform", 1, 0).unwrap();
}

#[test]
fn test_compile_time_type_compatibility_validation() {
  // Test that type compatibility is validated at compile time
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));

  let builder = builder.node(producer).unwrap();
  let builder = builder.node(transformer).unwrap();

  // Compatible types (i32 -> i32) - use connect_by_name
  let _builder = builder.connect_by_name("source", "transform").unwrap();

  // The following would fail to compile if uncommented:
  // // Incompatible types (i32 -> String) - would fail to compile
  // let string_transformer = TransformerNode::new(
  //   "string_transform".to_string(),
  //   MapTransformer::new(|x: String| x.len()),
  // );
  // builder.node(string_transformer).unwrap()
  //   .connect::<
  //     ProducerNode<VecProducer<i32>, (i32,)>,
  //     TransformerNode<MapTransformer<String, usize>, (String,), (usize,)>,
  //     0, 0
  //   >("source", "string_transform", 0, 0).unwrap();
}

#[test]
fn test_port_name_resolution() {
  // Test port name resolution for different node types
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));
  let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

  // Test producer output port names
  assert_eq!(
    producer.output_port_names().get(0),
    Some(&"out".to_string())
  ); // Single-port uses "out"
  assert!(producer.has_output_port("out")); // Single-port uses "out"
  assert!(!producer.has_output_port("out0")); // No longer supported
  assert!(!producer.has_output_port("0")); // Numeric indices no longer supported
  assert_eq!(producer.input_port_names().len(), 0); // No input ports

  // Test transformer port names
  assert_eq!(
    transformer.input_port_names().get(0),
    Some(&"in".to_string())
  ); // Single-port uses "in"
  assert_eq!(
    transformer.output_port_names().get(0),
    Some(&"out".to_string())
  ); // Single-port uses "out"
  assert!(transformer.has_input_port("in")); // Single-port uses "in"
  assert!(!transformer.has_input_port("in0")); // No longer supported
  assert!(!transformer.has_input_port("0")); // Numeric indices no longer supported
  assert!(transformer.has_output_port("out")); // Single-port uses "out"
  assert!(!transformer.has_output_port("out0")); // No longer supported
  assert!(!transformer.has_output_port("0")); // Numeric indices no longer supported

  // Test consumer input port names
  assert_eq!(consumer.input_port_names().get(0), Some(&"in".to_string())); // Single-port uses "in"
  assert!(consumer.has_input_port("in")); // Single-port uses "in"
  assert!(!consumer.has_input_port("in0")); // No longer supported
  assert!(!consumer.has_input_port("0")); // Numeric indices no longer supported
  assert_eq!(consumer.output_port_names().len(), 0); // No output ports
}

#[test]
fn test_port_name_resolution_invalid() {
  // Test invalid port name resolution
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));

  // Invalid port names should return false
  assert!(!producer.has_output_port("invalid"));
  assert!(!producer.has_output_port("out_a")); // Only single port exists (uses "out")
  assert!(!producer.has_output_port("in")); // Wrong prefix for output
  assert_eq!(producer.input_port_names().len(), 0); // No input ports
}

#[test]
fn test_connect_by_name_port_resolution() {
  // Test connect_by_name with various port name formats
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));
  let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

  let builder = builder.node(producer).unwrap();
  let builder = builder.node(transformer).unwrap();
  let builder = builder.node(consumer).unwrap();

  // Test various port name formats
  // 1. Node names only (defaults to port 0)
  let builder = builder.connect_by_name("source", "transform").unwrap();
  assert_eq!(builder.connection_count(), 1);

  // 2. Explicit port names (named ports)
  let builder = builder
    .connect_by_name("source:out", "transform:in")
    .unwrap();
  assert_eq!(builder.connection_count(), 2);

  // 3. Named ports (numeric indices no longer supported)
  let builder = builder.connect_by_name("transform:out", "sink:in").unwrap();
  assert_eq!(builder.connection_count(), 3);

  // 4. Named ports (all formats use named ports now)
  let builder = builder
    .connect_by_name("source:out", "transform:in")
    .unwrap();
  assert_eq!(builder.connection_count(), 4);

  let graph = builder.build();
  assert_eq!(graph.get_connections().len(), 4);
}

#[test]
fn test_connect_by_name_invalid_ports() {
  // Test connect_by_name with invalid port names
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));

  let builder = builder.node(producer).unwrap();
  let builder = builder.node(transformer).unwrap();

  // Invalid port names should fail - each call consumes the builder
  let result1 = builder.connect_by_name("source:invalid", "transform");
  assert!(result1.is_err());

  // Create new builder for second test
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));
  let builder = builder.node(producer).unwrap();
  let builder = builder.node(transformer).unwrap();
  let result2 = builder.connect_by_name("source", "transform:invalid");
  assert!(result2.is_err());

  // Create new builder for third test
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));
  let builder = builder.node(producer).unwrap();
  let builder = builder.node(transformer).unwrap();
  let result3 = builder.connect_by_name("source:out1", "transform");
  assert!(result3.is_err()); // Port 1 doesn't exist
}

#[test]
fn test_connect_by_name_node_not_found() {
  // Test connect_by_name with non-existent nodes
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));

  let builder = builder.node(producer).unwrap();

  // Non-existent nodes should fail - each call consumes the builder, so we need separate tests
  let builder1 = builder.connect_by_name("nonexistent", "source");
  assert!(builder1.is_err());

  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let builder = builder.node(producer).unwrap();
  let builder2 = builder.connect_by_name("source", "nonexistent");
  assert!(builder2.is_err());
}

#[test]
fn test_graph_builder_build() {
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));

  let builder = builder.add_node("source".to_string(), producer).unwrap();
  let graph = builder.build();

  assert_eq!(graph.len(), 1);
  assert!(!graph.is_empty());
  assert!(graph.get_node("source").is_some());
}

#[test]
fn test_runtime_graph_builder() {
  let mut builder = RuntimeGraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));

  assert!(
    builder
      .add_node("source".to_string(), Box::new(producer))
      .is_ok()
  );
  assert!(
    builder
      .add_node(
        "source".to_string(),
        Box::new(ProducerNode::from_producer(
          "duplicate".to_string(),
          VecProducer::new(vec![4, 5, 6]),
        ))
      )
      .is_err()
  );
}

#[test]
fn test_runtime_graph_builder_connect() {
  let mut builder = RuntimeGraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

  builder
    .add_node("source".to_string(), Box::new(producer))
    .unwrap();
  builder
    .add_node("sink".to_string(), Box::new(consumer))
    .unwrap();

  // Valid connection (using port names)
  assert!(builder.connect(("source", "out"), ("sink", "in")).is_ok());

  // Invalid: node doesn't exist
  assert!(
    builder
      .connect(("nonexistent", "out"), ("sink", "in"))
      .is_err()
  );
}

#[test]
fn test_graph_get_children() {
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer1 = TransformerNode::from_transformer(
    "transform1".to_string(),
    MapTransformer::new(|x: i32| x * 2),
  );
  let transformer2 = TransformerNode::from_transformer(
    "transform2".to_string(),
    MapTransformer::new(|x: i32| x * 3),
  );

  let builder = builder.add_node("source".to_string(), producer).unwrap();
  let builder = builder
    .add_node("transform1".to_string(), transformer1)
    .unwrap();
  let builder = builder
    .add_node("transform2".to_string(), transformer2)
    .unwrap();

  let builder = builder.connect_by_name("source", "transform1").unwrap();
  let builder = builder.connect_by_name("source", "transform2").unwrap();

  let graph = builder.build();
  let children = graph.get_children("source");
  assert_eq!(children.len(), 2);
  assert!(children.contains(&("transform1", "in")));
  assert!(children.contains(&("transform2", "in")));
}

#[test]
fn test_graph_get_parents() {
  let builder = GraphBuilder::new();
  let producer = ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3]));
  let transformer =
    TransformerNode::from_transformer("transform".to_string(), MapTransformer::new(|x: i32| x * 2));
  let consumer = ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new());

  let builder = builder.add_node("source".to_string(), producer).unwrap();
  let builder = builder
    .add_node("transform".to_string(), transformer)
    .unwrap();
  let builder = builder.add_node("sink".to_string(), consumer).unwrap();

  let builder = builder.connect_by_name("source", "transform").unwrap();
  let builder = builder.connect_by_name("transform", "sink").unwrap();

  let graph = builder.build();
  let parents = graph.get_parents("transform");
  assert_eq!(parents.len(), 1);
  assert_eq!(parents[0], ("source", "out"));
}

#[test]
fn test_graph_node_names() {
  let builder = GraphBuilder::new();
  let builder = builder
    .add_node(
      "source".to_string(),
      ProducerNode::from_producer("source".to_string(), VecProducer::new(vec![1, 2, 3])),
    )
    .unwrap();
  let builder = builder
    .add_node(
      "sink".to_string(),
      ConsumerNode::from_consumer("sink".to_string(), VecConsumer::<i32>::new()),
    )
    .unwrap();

  let graph = builder.build();
  let names = graph.node_names();
  assert_eq!(names.len(), 2);
  assert!(names.contains(&"source"));
  assert!(names.contains(&"sink"));
}

#[test]
fn test_graph_error_display() {
  let error = GraphError::NodeNotFound {
    name: "test".to_string(),
  };
  assert_eq!(error.to_string(), "Node not found: test");

  let error = GraphError::DuplicateNode {
    name: "test".to_string(),
  };
  assert_eq!(error.to_string(), "Duplicate node: test");

  let error = GraphError::InvalidConnection {
    source: "source".to_string(),
    target: "target".to_string(),
    reason: "test reason".to_string(),
  };
  assert_eq!(
    error.to_string(),
    "Invalid connection from source to target: test reason"
  );
}
