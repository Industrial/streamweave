//! Tests for WhileLoopNode

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::while_loop_node::{WhileLoopConfig, WhileLoopNode, while_loop_config};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams() -> (TestSender, TestSender, TestSender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (data_tx, data_rx) = mpsc::channel(10);
  let (break_tx, break_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "condition".to_string(),
    Box::pin(ReceiverStream::new(break_rx)) as crate::node::InputStream,
  );

  (config_tx, data_tx, break_tx, inputs)
}

#[tokio::test]
async fn test_while_loop_node_creation() {
  let node = WhileLoopNode::new("test_while_loop".to_string());
  assert_eq!(node.name(), "test_while_loop");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("condition"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("break"));
  assert!(node.has_output_port("error"));
  assert!(!node.has_config());
}

#[tokio::test]
async fn test_while_loop_node_basic_loop() {
  let node = WhileLoopNode::new("test_while_loop".to_string());

  // Create a config that loops while value is less than 5
  let config: WhileLoopConfig = while_loop_config(
    |value| async move {
      if let Ok(arc_i32) = value.downcast::<i32>() {
        Ok(*arc_i32 < 5)
      } else {
        Err("Expected i32".to_string())
      }
    },
    100, // max iterations
  );

  let (config_tx, data_tx, _break_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(Arc::new(config)) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data: value 0, which should loop until it becomes 5
  // Actually, the condition checks if value < 5, so it will loop while true
  // But we're not modifying the value in the loop, so it will loop forever until max iterations
  // Let's use a different approach: send a value that will eventually become false
  // Actually, for a proper test, we need the condition to change. But the node doesn't modify values.
  // So let's test with a value that starts as true but we'll use max iterations to prevent infinite loop.
  // Actually, let's just test that it loops and exits when condition becomes false.
  // But the condition function receives the same value each time, so it will always return the same result.
  // This means if condition is true, it loops forever (until max iterations).
  // If condition is false, it exits immediately.

  // Test with value 3 (< 5, so condition is true, will loop until max iterations)
  let _ = data_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Wait for processing
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  // Check error output (should have max iterations error)
  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result
        && let Ok(arc_string) = item.downcast::<String>() {
          errors.push(arc_string.clone());
        }
    }
    _ = &mut timeout => {},
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("Maximum iterations"));
}

#[tokio::test]
async fn test_while_loop_node_exits_when_condition_false() {
  let node = WhileLoopNode::new("test_while_loop".to_string());

  // Create a config that always returns false (so loop exits immediately)
  let config: WhileLoopConfig = while_loop_config(
    |_value| async move {
      Ok(false) // Condition is false, so loop exits immediately
    },
    100,
  );

  let (config_tx, data_tx, _break_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(Arc::new(config)) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data
  let _ = data_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result
        && let Ok(arc_i32) = item.downcast::<i32>() {
          results.push(*arc_i32);
        }
    }
    _ = &mut timeout => {},
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], 42);
}

#[tokio::test]
async fn test_while_loop_node_break_signal() {
  let node = WhileLoopNode::new("test_while_loop".to_string());

  // Create a config that always returns true (infinite loop)
  let config: WhileLoopConfig = while_loop_config(
    |_value| async move {
      Ok(true) // Condition is always true, so loop continues
    },
    1000, // High max iterations
  );

  let (config_tx, data_tx, break_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(Arc::new(config)) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data
  let _ = data_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send break signal immediately
  let _ = break_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Wait for processing
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Check break output
  let break_stream = outputs.remove("break").unwrap();
  let mut results = Vec::new();
  let mut stream = break_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result
        && let Ok(arc_i32) = item.downcast::<i32>() {
          results.push(*arc_i32);
        }
    }
    _ = &mut timeout => {},
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], 10);
}

#[tokio::test]
async fn test_while_loop_node_error_on_no_config() {
  let node = WhileLoopNode::new("test_while_loop".to_string());

  let (_config_tx, data_tx, _break_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send data without configuration
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect error
  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result
        && let Ok(arc_string) = item.downcast::<String>() {
          errors.push(arc_string.clone());
        }
    }
    _ = &mut timeout => {},
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("No configuration set"));
}

#[tokio::test]
async fn test_while_loop_node_condition_error() {
  let node = WhileLoopNode::new("test_while_loop".to_string());

  // Create a config that returns an error
  let config: WhileLoopConfig = while_loop_config(
    |_value| async move { Err("Condition evaluation failed".to_string()) },
    100,
  );

  let (config_tx, data_tx, _break_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(Arc::new(config)) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data
  let _ = data_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect error
  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result
        && let Ok(arc_string) = item.downcast::<String>() {
          errors.push(arc_string.clone());
        }
    }
    _ = &mut timeout => {},
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("Condition evaluation failed"));
}
