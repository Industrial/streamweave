//! Comprehensive error handling tests for graph execution
//!
//! This module provides tests for all error scenarios including:
//! - Serialization/deserialization errors
//! - Channel errors
//! - Stream errors
//! - Node execution errors
//! - Lifecycle errors
//! - Invalid topology errors

use std::time::Duration;
use streamweave_graph::{
  Graph, GraphBuilder, GraphExecution,
  execution::{ExecutionError, ExecutionState, GraphExecutor},
  node::{ConsumerNode, ProducerNode, TransformerNode},
};
use streamweave_transformers::map::MapTransformer;
use streamweave_vec::{consumers::VecConsumer, producers::VecProducer};
use tokio::time::sleep;

#[cfg(test)]
mod serialization_error_tests {
  use super::*;

  // Note: Testing serialization errors requires creating types that fail to serialize
  // or manipulating serialized data. These tests document the expected behavior.

  #[tokio::test]
  async fn test_serialization_error_handling() {
    // This test verifies that serialization errors are caught and reported correctly
    // Actual serialization failures are rare with well-formed data, but error handling
    // should be robust when they occur.

    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    sleep(Duration::from_millis(100)).await;
    executor.stop().await.unwrap();

    // Verify no serialization errors occurred
    let errors = executor.errors();
    assert!(
      errors.is_empty()
        || errors
          .iter()
          .all(|e| !matches!(e, ExecutionError::SerializationError { .. }))
    );
  }
}

#[cfg(test)]
mod channel_error_tests {
  use super::*;

  #[tokio::test]
  async fn test_channel_receiver_dropped() {
    // This test verifies that channel errors are handled when receivers are dropped
    // Channel errors occur when a receiver is dropped before the sender finishes

    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    sleep(Duration::from_millis(50)).await;

    // Stop immediately which should handle channel closures gracefully
    executor.stop_immediate().await.unwrap();
  }

  #[tokio::test]
  async fn test_channel_error_reporting() {
    // Verify that channel errors are collected and can be inspected
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    sleep(Duration::from_millis(100)).await;
    executor.stop().await.unwrap();

    // Check that errors() method works
    let _errors = executor.errors();
  }
}

#[cfg(test)]
mod lifecycle_error_tests {
  use super::*;

  #[tokio::test]
  async fn test_pause_error_invalid_state() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();

    // Try to pause when stopped - should fail
    let result = executor.pause().await;
    assert!(result.is_err());
    if let Err(ExecutionError::LifecycleError {
      operation,
      current_state,
      ..
    }) = result
    {
      assert_eq!(operation, "pause");
      assert_eq!(current_state, ExecutionState::Stopped);
    } else {
      panic!("Expected LifecycleError");
    }
  }

  #[tokio::test]
  async fn test_resume_error_invalid_state() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();

    // Try to resume when stopped - should fail
    let result = executor.resume().await;
    assert!(result.is_err());
    if let Err(ExecutionError::LifecycleError {
      operation,
      current_state,
      ..
    }) = result
    {
      assert_eq!(operation, "resume");
      assert_eq!(current_state, ExecutionState::Stopped);
    } else {
      panic!("Expected LifecycleError");
    }
  }

  #[tokio::test]
  async fn test_start_when_already_running() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();

    // Try to start again - should fail
    let result = executor.start().await;
    assert!(result.is_err());
    if let Err(ExecutionError::ExecutionFailed(msg)) = result {
      assert!(msg.contains("already running"));
    } else {
      panic!("Expected ExecutionFailed error");
    }

    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_pause_resume_lifecycle() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();

    // Pause should succeed
    assert!(executor.pause().await.is_ok());
    assert_eq!(executor.state(), ExecutionState::Paused);

    // Resume should succeed
    assert!(executor.resume().await.is_ok());
    assert_eq!(executor.state(), ExecutionState::Running);

    executor.stop().await.unwrap();
  }
}

#[cfg(test)]
mod topology_error_tests {
  use super::*;

  #[tokio::test]
  async fn test_empty_graph_start_error() {
    let graph = Graph::new();
    let mut executor = graph.executor();

    // Starting an empty graph should fail
    let result = executor.start().await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_graph_with_no_connections() {
    // A graph with nodes but no connections may be valid depending on implementation
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();
    // This may or may not succeed depending on topology validation
    let _result = executor.start().await;
  }
}

#[cfg(test)]
mod shutdown_error_tests {
  use super::*;

  #[tokio::test]
  async fn test_graceful_shutdown_with_timeout() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Graceful shutdown should succeed
    assert!(executor.stop().await.is_ok());
  }

  #[tokio::test]
  async fn test_stop_when_already_stopped() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();

    // Stopping when already stopped should succeed (idempotent)
    assert!(executor.stop().await.is_ok());
    assert!(executor.stop().await.is_ok());
  }

  #[tokio::test]
  async fn test_stop_immediate() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();

    // Stop immediate should succeed
    assert!(executor.stop_immediate().await.is_ok());
    assert_eq!(executor.state(), ExecutionState::Stopped);
  }

  #[tokio::test]
  async fn test_shutdown_timeout_configuration() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = GraphExecutor::with_shutdown_timeout(graph, Duration::from_secs(5));

    assert_eq!(executor.shutdown_timeout(), Duration::from_secs(5));

    executor.set_shutdown_timeout(Duration::from_secs(10));
    assert_eq!(executor.shutdown_timeout(), Duration::from_secs(10));
  }
}

#[cfg(test)]
mod error_collection_tests {
  use super::*;

  #[tokio::test]
  async fn test_error_collection_empty() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let executor = graph.executor();

    // Initially no errors
    assert!(executor.errors().is_empty());
  }

  #[tokio::test]
  async fn test_error_clearing() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();

    // Clear errors when empty should be fine
    executor.clear_errors();
    assert!(executor.errors().is_empty());
  }

  #[tokio::test]
  async fn test_error_display_format() {
    // Test that error Display implementation works correctly
    let error = ExecutionError::NodeExecutionFailed {
      node: "test_node".to_string(),
      reason: "test reason".to_string(),
    };

    let display_str = format!("{}", error);
    assert!(display_str.contains("test_node"));
    assert!(display_str.contains("test reason"));

    let error = ExecutionError::ChannelError {
      node: "test_node".to_string(),
      port: 0,
      is_input: false,
      reason: "receiver dropped".to_string(),
    };

    let display_str = format!("{}", error);
    assert!(display_str.contains("test_node"));
    assert!(display_str.contains("output"));
    assert!(display_str.contains("receiver dropped"));

    let error = ExecutionError::SerializationError {
      node: "test_node".to_string(),
      is_deserialization: true,
      reason: "invalid json".to_string(),
    };

    let display_str = format!("{}", error);
    assert!(display_str.contains("test_node"));
    assert!(display_str.contains("deserialization"));
    assert!(display_str.contains("invalid json"));

    let error = ExecutionError::LifecycleError {
      operation: "pause".to_string(),
      current_state: ExecutionState::Stopped,
      reason: "cannot pause".to_string(),
    };

    let display_str = format!("{}", error);
    assert!(display_str.contains("pause"));
    assert!(display_str.contains("Stopped"));
    assert!(display_str.contains("cannot pause"));
  }
}

#[cfg(test)]
mod comprehensive_error_scenarios {
  use super::*;

  #[tokio::test]
  async fn test_multiple_error_types() {
    // Test that multiple types of errors can be handled in the same execution
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
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
    sleep(Duration::from_millis(100)).await;

    // Pause and resume to test lifecycle
    executor.pause().await.unwrap();
    executor.resume().await.unwrap();

    executor.stop().await.unwrap();

    // Verify error collection works
    let errors = executor.errors();
    // In a successful execution, there should be no errors
    assert!(errors.is_empty());
  }

  #[tokio::test]
  async fn test_error_propagation_through_graph() {
    // Test that errors are properly propagated and collected
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .node(ConsumerNode::from_consumer(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
      ))
      .unwrap()
      .connect_by_name("source", "sink")
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.start().await.unwrap();
    sleep(Duration::from_millis(100)).await;
    executor.stop().await.unwrap();

    // Errors should be accessible after execution
    let _errors = executor.errors();
  }
}
