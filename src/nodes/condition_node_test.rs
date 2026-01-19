//! # Condition Node Test Suite
//!
//! Comprehensive test suite for the ConditionNode implementation.

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::condition_node::{ConditionConfig, ConditionNode, condition_config};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams() -> (TestSender, TestSender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (data_tx, data_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as crate::node::InputStream,
  );

  (config_tx, data_tx, inputs)
}

#[tokio::test]
async fn test_condition_node_creation() {
  let node = ConditionNode::new("test_condition".to_string());
  assert_eq!(node.name(), "test_condition");
  assert!(!node.has_config());
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("true"));
  assert!(node.has_output_port("false"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_condition_node_routing() {
  let node = ConditionNode::new("test_condition".to_string());

  // Create a config that routes positive numbers to "true" and negative to "false"
  let config: ConditionConfig = condition_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(*arc_i32 >= 0)
    } else {
      Err("Expected i32".to_string())
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data: positive, negative, positive
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(-3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results from "true" port
  let true_stream = outputs.remove("true").unwrap();
  let mut true_results = Vec::new();
  let mut true_stream = true_stream;

  for _ in 0..2 {
    tokio::select! {
      result = true_stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            true_results.push(*arc_i32);
          }
        } else {
          break;
        }
      }
      _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
        break;
      }
    }
  }

  // Collect results from "false" port
  let false_stream = outputs.remove("false").unwrap();
  let mut false_results = Vec::new();
  let mut false_stream = false_stream;

  tokio::select! {
    result = false_stream.next() => {
      if let Some(item) = result
        && let Ok(arc_i32) = item.downcast::<i32>() {
          false_results.push(*arc_i32);
        }
    }
    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
  }

  // Verify results
  assert_eq!(true_results.len(), 2);
  assert!(true_results.contains(&5));
  assert!(true_results.contains(&10));

  assert_eq!(false_results.len(), 1);
  assert!(false_results.contains(&-3));
}

#[tokio::test]
async fn test_condition_node_error_on_no_config() {
  let node = ConditionNode::new("test_condition".to_string());

  let (_config_tx, data_tx, inputs) = create_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send data without configuration
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Check error output
  let error_stream = outputs.remove("error").unwrap();
  let mut error_results = Vec::new();
  let mut stream = error_stream;

  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result
        && let Ok(arc_str) = item.downcast::<String>() {
          error_results.push(arc_str.clone());
        }
    }
    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
  }

  assert!(!error_results.is_empty());
  assert!(error_results[0].contains("No configuration set"));
}
