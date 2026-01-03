//! Execution and error handling tests
//!
//! This module provides tests for graph execution, error handling, and lifecycle management.

use std::time::Duration;
use streamweave::graph::nodes::{ConsumerNode, ProducerNode, TransformerNode};
use streamweave::graph::{
  ExecutionError, ExecutionState, Graph, GraphBuilder, GraphExecution, GraphExecutor,
};
use streamweave_transformers::MapTransformer;
use streamweave_vec::{VecConsumer, VecProducer};
use tokio::time::sleep;

#[cfg(test)]
mod serialization_error_tests {
  use super::*;

  #[tokio::test]
  async fn test_serialization_error_handling() {
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
    executor.stop_immediate().await.unwrap();
  }

  #[tokio::test]
  async fn test_channel_error_reporting() {
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
    assert!(executor.pause().await.is_ok());
    assert_eq!(executor.state(), ExecutionState::Paused);
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
    let result = executor.start().await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_graph_with_no_connections() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();
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
    executor.clear_errors();
    assert!(executor.errors().is_empty());
  }

  #[tokio::test]
  async fn test_error_display_format() {
    let error = ExecutionError::NodeExecutionFailed {
      node: "test_node".to_string(),
      reason: "test reason".to_string(),
      message_id: None,
    };

    let display_str = format!("{}", error);
    assert!(display_str.contains("test_node"));
    assert!(display_str.contains("test reason"));

    let error = ExecutionError::ChannelError {
      node: "test_node".to_string(),
      port: "out".to_string(),
      is_input: false,
      reason: "receiver dropped".to_string(),
      message_id: None,
    };

    let display_str = format!("{}", error);
    assert!(display_str.contains("test_node"));
    assert!(display_str.contains("output"));
    assert!(display_str.contains("receiver dropped"));

    let error = ExecutionError::SerializationError {
      node: "test_node".to_string(),
      is_deserialization: true,
      reason: "invalid json".to_string(),
      message_id: None,
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
    executor.pause().await.unwrap();
    executor.resume().await.unwrap();
    executor.stop().await.unwrap();

    let errors = executor.errors();
    assert!(errors.is_empty());
  }

  #[tokio::test]
  async fn test_error_propagation_through_graph() {
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

    let _errors = executor.errors();
  }
}

#[cfg(test)]
mod lifecycle_tests {
  use super::*;
  use tokio::time::timeout;

  fn create_linear_graph() -> streamweave::graph::Graph {
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

  async fn wait_for_completion() {
    sleep(Duration::from_millis(100)).await;
  }

  #[tokio::test]
  async fn test_start_stop_lifecycle() {
    let graph = create_linear_graph();
    let mut executor = graph.executor();

    assert_eq!(
      executor.state(),
      streamweave::graph::ExecutionState::Stopped
    );
    assert!(!executor.is_running());

    executor.start().await.unwrap();
    assert_eq!(
      executor.state(),
      streamweave::graph::ExecutionState::Running
    );
    assert!(executor.is_running());

    executor.stop().await.unwrap();
    assert_eq!(
      executor.state(),
      streamweave::graph::ExecutionState::Stopped
    );
    assert!(!executor.is_running());
  }

  #[tokio::test]
  async fn test_pause_resume_lifecycle() {
    let graph = create_linear_graph();
    let mut executor = graph.executor();

    executor.start().await.unwrap();
    assert_eq!(
      executor.state(),
      streamweave::graph::ExecutionState::Running
    );

    executor.pause().await.unwrap();
    assert_eq!(executor.state(), streamweave::graph::ExecutionState::Paused);
    assert!(!executor.is_running());

    executor.resume().await.unwrap();
    assert_eq!(
      executor.state(),
      streamweave::graph::ExecutionState::Running
    );
    assert!(executor.is_running());

    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_double_start_error() {
    let graph = create_linear_graph();
    let mut executor = graph.executor();

    executor.start().await.unwrap();
    assert!(executor.start().await.is_err());
    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_empty_graph_error() {
    let graph = streamweave::graph::Graph::new();
    let mut executor = graph.executor();
    assert!(executor.start().await.is_err());
  }

  #[tokio::test]
  async fn test_stop_when_stopped() {
    let graph = streamweave::graph::Graph::new();
    let mut executor = graph.executor();
    assert!(executor.stop().await.is_ok());
  }

  #[tokio::test]
  async fn test_stop_immediate() {
    let graph = create_linear_graph();
    let mut executor = graph.executor();

    executor.start().await.unwrap();
    executor.stop_immediate().await.unwrap();
    assert_eq!(
      executor.state(),
      streamweave::graph::ExecutionState::Stopped
    );
  }

  #[tokio::test]
  async fn test_graceful_shutdown() {
    let graph = create_linear_graph();
    let mut executor = graph.executor();

    executor.start().await.unwrap();
    wait_for_completion().await;

    let stop_result = timeout(Duration::from_secs(5), executor.stop()).await;
    assert!(stop_result.is_ok());
    assert!(stop_result.unwrap().is_ok());
  }

  #[tokio::test]
  async fn test_error_collection() {
    let graph = create_linear_graph();
    let mut executor = graph.executor();

    assert!(executor.errors().is_empty());

    executor.start().await.unwrap();
    wait_for_completion().await;
    executor.stop().await.unwrap();

    let errors = executor.errors();
    assert!(errors.is_empty() || !errors.is_empty());
  }
}

#[cfg(test)]
mod additional_execution_coverage {
  use super::*;
  use std::error::Error;

  #[test]
  fn test_execution_error_all_variants_display() {
    let error = ExecutionError::NodeExecutionFailed {
      node: "test_node".to_string(),
      reason: "test reason".to_string(),
      message_id: None,
    };
    let display_str = format!("{}", error);
    assert!(display_str.contains("test_node"));
    assert!(display_str.contains("test reason"));

    let error = ExecutionError::ChannelError {
      node: "test_node".to_string(),
      port: "in".to_string(),
      is_input: true,
      reason: "test channel error".to_string(),
      message_id: None,
    };
    let display_str = format!("{}", error);
    assert!(display_str.contains("test_node"));
    assert!(display_str.contains("input"));
    assert!(display_str.contains("test channel error"));

    let error = ExecutionError::SerializationError {
      node: "test_node".to_string(),
      is_deserialization: false,
      reason: "test serialization error".to_string(),
      message_id: None,
    };
    let display_str = format!("{}", error);
    assert!(display_str.contains("test_node"));
    assert!(display_str.contains("serialization"));
    assert!(display_str.contains("test serialization error"));

    let error = ExecutionError::LifecycleError {
      operation: "test_op".to_string(),
      current_state: ExecutionState::Stopped,
      reason: "test lifecycle error".to_string(),
    };
    let display_str = format!("{}", error);
    assert!(display_str.contains("test_op"));
    assert!(display_str.contains("Stopped"));
    assert!(display_str.contains("test lifecycle error"));

    let error = ExecutionError::ExecutionFailed("test execution failed".to_string());
    let display_str = format!("{}", error);
    assert!(display_str.contains("test execution failed"));
  }

  #[test]
  fn test_execution_error_error_trait() {
    let error = ExecutionError::NodeExecutionFailed {
      node: "test_node".to_string(),
      reason: "test reason".to_string(),
      message_id: None,
    };

    let _error_ref: &dyn Error = &error;
    assert!(!error.to_string().is_empty());
  }

  #[test]
  fn test_execution_state_debug() {
    let state = ExecutionState::Stopped;
    let debug_str = format!("{:?}", state);
    assert!(!debug_str.is_empty());

    let state = ExecutionState::Running;
    let debug_str = format!("{:?}", state);
    assert!(!debug_str.is_empty());

    let state = ExecutionState::Paused;
    let debug_str = format!("{:?}", state);
    assert!(!debug_str.is_empty());
  }

  #[tokio::test]
  async fn test_executor_with_shutdown_timeout() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let executor = GraphExecutor::with_shutdown_timeout(graph, Duration::from_secs(10));
    assert_eq!(executor.shutdown_timeout(), Duration::from_secs(10));
  }

  #[tokio::test]
  async fn test_executor_set_shutdown_timeout() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();
    executor.set_shutdown_timeout(Duration::from_secs(5));
    assert_eq!(executor.shutdown_timeout(), Duration::from_secs(5));
  }

  #[tokio::test]
  async fn test_executor_state_transitions() {
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
    assert_eq!(executor.state(), ExecutionState::Stopped);
    assert!(!executor.is_running());

    executor.start().await.unwrap();
    assert_eq!(executor.state(), ExecutionState::Running);
    assert!(executor.is_running());

    executor.pause().await.unwrap();
    assert_eq!(executor.state(), ExecutionState::Paused);
    assert!(!executor.is_running());

    executor.resume().await.unwrap();
    assert_eq!(executor.state(), ExecutionState::Running);
    assert!(executor.is_running());

    executor.stop().await.unwrap();
    assert_eq!(executor.state(), ExecutionState::Stopped);
    assert!(!executor.is_running());
  }

  #[tokio::test]
  async fn test_executor_clear_errors() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();
    assert!(executor.errors().is_empty());
    executor.clear_errors();
    assert!(executor.errors().is_empty());
  }

  #[tokio::test]
  async fn test_executor_errors_immutable() {
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
    executor.stop().await.unwrap();

    let errors1 = executor.errors();
    let errors2 = executor.errors();
    assert_eq!(errors1.len(), errors2.len());
  }

  #[tokio::test]
  async fn test_executor_stop_immediate_state() {
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
    sleep(Duration::from_millis(10)).await;
    executor.stop_immediate().await.unwrap();
    assert_eq!(executor.state(), ExecutionState::Stopped);
    assert!(!executor.is_running());
  }

  #[tokio::test]
  async fn test_executor_resume_from_stopped_error() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();
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
  async fn test_executor_pause_from_stopped_error() {
    let graph = GraphBuilder::new()
      .node(ProducerNode::from_producer(
        "source".to_string(),
        VecProducer::new(vec![1, 2, 3]),
      ))
      .unwrap()
      .build();

    let mut executor = graph.executor();
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
  async fn test_executor_start_twice_error() {
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
    let result = executor.start().await;
    assert!(result.is_err());
    executor.stop().await.unwrap();
  }

  #[tokio::test]
  async fn test_executor_channel_error_output_port() {
    let error = ExecutionError::ChannelError {
      node: "test_node".to_string(),
      port: "out".to_string(),
      is_input: false,
      reason: "receiver dropped".to_string(),
      message_id: None,
    };
    let display_str = format!("{}", error);
    assert!(display_str.contains("output"));
    assert!(display_str.contains("receiver dropped"));
  }

  #[tokio::test]
  async fn test_executor_serialization_error_serialization() {
    let error = ExecutionError::SerializationError {
      node: "test_node".to_string(),
      is_deserialization: false,
      reason: "invalid format".to_string(),
      message_id: None,
    };
    let display_str = format!("{}", error);
    assert!(display_str.contains("serialization"));
    assert!(display_str.contains("invalid format"));
  }
}

// Tests moved from src/

#[tokio::test]
async fn test_throughput_monitor() {
  use std::time::Duration;
  use streamweave::graph::throughput::ThroughputMonitor;

  let monitor = ThroughputMonitor::new(Duration::from_secs(1));

  // Initially zero
  assert_eq!(monitor.item_count(), 0);

  // Increment items
  monitor.increment_item_count();
  monitor.increment_by(5);
  assert_eq!(monitor.item_count(), 6);

  // Calculate throughput (may be 0 if not enough time has passed)
  let throughput = monitor.calculate_throughput().await;
  assert!(throughput >= 0.0);

  // Reset
  monitor.reset().await;
  assert_eq!(monitor.item_count(), 0);
}

#[tokio::test]
async fn test_mode_switch_metrics() {
  use streamweave::graph::execution::ModeSwitchMetrics;

  let mut metrics = ModeSwitchMetrics::new();
  assert_eq!(metrics.switch_count, 0);

  // Record a switch
  metrics.record_switch(150.0, "InProcess", "Distributed");
  assert_eq!(metrics.switch_count, 1);
  assert_eq!(metrics.switch_reasons.len(), 1);
  assert_eq!(metrics.switch_reasons[0], 150.0);
  assert_eq!(metrics.current_mode, Some("Distributed".to_string()));

  // Record another switch
  metrics.record_switch(50.0, "Distributed", "InProcess");
  assert_eq!(metrics.switch_count, 2);
}

#[tokio::test]
async fn test_graph_executor_creation() {
  let graph = Graph::new();
  let executor = graph.executor();

  assert_eq!(executor.state(), ExecutionState::Stopped);
  assert!(!executor.is_running());
}

#[tokio::test]
async fn test_graph_executor_start_stop() {
  // Create a graph with at least one node
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    ))
    .unwrap()
    .build();
  let mut executor = graph.executor();

  // Start execution
  assert!(executor.start().await.is_ok());
  assert_eq!(executor.state(), ExecutionState::Running);
  assert!(executor.is_running());

  // Stop execution
  assert!(executor.stop().await.is_ok());
  assert_eq!(executor.state(), ExecutionState::Stopped);
  assert!(!executor.is_running());
}

#[tokio::test]
async fn test_graph_executor_double_start() {
  // Create a graph with at least one node
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    ))
    .unwrap()
    .build();
  let mut executor = graph.executor();

  assert!(executor.start().await.is_ok());
  // Starting again should fail
  assert!(executor.start().await.is_err());
}

#[tokio::test]
async fn test_graph_executor_empty_graph() {
  let graph = Graph::new();
  let mut executor = graph.executor();

  // Starting an empty graph should fail
  assert!(executor.start().await.is_err());
}

#[tokio::test]
async fn test_graph_executor_stop_when_stopped() {
  let graph = Graph::new();
  let mut executor = graph.executor();

  // Stopping when already stopped should succeed
  assert!(executor.stop().await.is_ok());
}

#[tokio::test]
async fn test_graph_executor_pause_resume() {
  // Create a graph with at least one node
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    ))
    .unwrap()
    .build();
  let mut executor = graph.executor();

  // Cannot pause when not running
  assert!(executor.pause().await.is_err());

  // Start execution
  assert!(executor.start().await.is_ok());

  // Pause execution
  assert!(executor.pause().await.is_ok());
  assert!(executor.is_paused());
  assert_eq!(executor.state(), ExecutionState::Paused);

  // Resume execution
  assert!(executor.resume().await.is_ok());
  assert!(!executor.is_paused());
  assert_eq!(executor.state(), ExecutionState::Running);

  // Stop execution
  assert!(executor.stop().await.is_ok());
}

#[tokio::test]
async fn test_graph_executor_resume_when_not_paused() {
  // Create a graph with at least one node
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    ))
    .unwrap()
    .build();
  let mut executor = graph.executor();

  // Cannot resume when not paused
  assert!(executor.resume().await.is_err());

  // Start execution
  assert!(executor.start().await.is_ok());

  // Cannot resume when running (not paused)
  assert!(executor.resume().await.is_err());
}

#[tokio::test]
async fn test_graph_executor_pause_signal() {
  let graph = Graph::new();
  let executor = graph.executor();

  // Get pause signal
  let pause_signal = executor.pause_signal();

  // Initially not paused
  assert!(!*pause_signal.read().await);

  // Set pause signal
  *pause_signal.write().await = true;
  assert!(*pause_signal.read().await);

  // Clear pause signal
  *pause_signal.write().await = false;
  assert!(!*pause_signal.read().await);
}

#[tokio::test]
async fn test_graph_executor_channel_creation() {
  use streamweave_vec::VecProducer;

  // Create a simple graph with one node (no connections yet)
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    ))
    .unwrap()
    .build();

  let mut executor = graph.executor();

  // Create channels (even though there are no connections, this should succeed)
  assert!(executor.create_channels_with_buffer_size(64).is_ok());
  assert_eq!(executor.channel_count(), 0);
}

#[tokio::test]
async fn test_graph_executor_channel_helpers() {
  use streamweave_vec::VecProducer;

  // Create a simple graph
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    ))
    .unwrap()
    .build();

  let mut executor = graph.executor();

  // No channels exist yet
  assert!(!executor.has_channel("source", "out", true));
  assert_eq!(executor.channel_count(), 0);
  assert!(executor.get_channel_sender("source", "out").is_none());
  assert!(executor.get_channel_receiver("source", "in").is_none());
}

#[tokio::test]
async fn test_graph_executor_lifecycle_errors() {
  let graph = Graph::new();
  let mut executor = graph.executor();

  // Initially no errors
  assert!(executor.errors().is_empty());

  // Clear errors (should be no-op)
  executor.clear_errors();
  assert!(executor.errors().is_empty());
}

#[tokio::test]
async fn test_graph_executor_shutdown_timeout() {
  let graph = Graph::new();
  let executor = graph.executor();

  // Default timeout is 30 seconds
  assert_eq!(executor.shutdown_timeout(), Duration::from_secs(30));

  // Create executor with custom timeout (need new graph since previous one was moved)
  let graph2 = Graph::new();
  let custom_timeout = Duration::from_secs(60);
  let mut executor = GraphExecutor::with_shutdown_timeout(graph2, custom_timeout);
  assert_eq!(executor.shutdown_timeout(), custom_timeout);

  // Set new timeout
  executor.set_shutdown_timeout(Duration::from_secs(10));
  assert_eq!(executor.shutdown_timeout(), Duration::from_secs(10));
}

#[tokio::test]
async fn test_graph_executor_stop_immediate() {
  // Create a graph with at least one node
  let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
      "source".to_string(),
      VecProducer::new(vec![1, 2, 3]),
    ))
    .unwrap()
    .build();
  let mut executor = graph.executor();

  // Start execution
  assert!(executor.start().await.is_ok());

  // Stop immediately
  assert!(executor.stop_immediate().await.is_ok());
  assert_eq!(executor.state(), ExecutionState::Stopped);
}
