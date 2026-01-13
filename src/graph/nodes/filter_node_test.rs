//! # Filter Node Test Suite
//!
//! Comprehensive test suite for the FilterNode implementation.

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::filter_node::{FilterConfig, FilterNode, filter_config};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams() -> (
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (data_tx, data_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, data_tx, inputs)
}

#[tokio::test]
async fn test_filter_node_creation() {
  let node = FilterNode::new("test_filter".to_string());
  assert_eq!(node.name(), "test_filter");
  assert!(!node.has_config());
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_filter_node_basic_filtering() {
  let node = FilterNode::new("test_filter".to_string());

  // Create a config that filters out negative numbers
  let config: FilterConfig = filter_config(|value| async move {
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

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  // Collect up to 2 items (only positive numbers should pass)
  for _ in 0..2 {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            results.push(*arc_i32);
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

  // Verify results: only positive numbers
  assert_eq!(results.len(), 2);
  assert!(results.contains(&5));
  assert!(results.contains(&10));
  assert!(!results.contains(&-3));
}

#[tokio::test]
async fn test_filter_node_error_on_no_config() {
  let node = FilterNode::new("test_filter".to_string());

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

#[tokio::test]
async fn test_filter_node_error_on_invalid_type() {
  let node = FilterNode::new("test_filter".to_string());

  // Create a config that only accepts i32
  let config: FilterConfig = filter_config(|value| async move {
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

  // Send invalid data (String instead of i32)
  let _ = data_tx
    .send(Arc::new("invalid".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert!(error_results[0].contains("Expected i32"));
}
