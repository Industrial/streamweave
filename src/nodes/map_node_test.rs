//! # Map Node Test Suite
//!
//! Comprehensive test suite for the MapNode implementation.

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::map_node::{MapConfig, MapNode, map_config};
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
async fn test_map_node_creation() {
  let node = MapNode::new("test_map".to_string());
  assert_eq!(node.name(), "test_map");
  assert!(!node.has_config());
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_map_node_basic_transformation() {
  let node = MapNode::new("test_map".to_string());

  // Create a config that multiplies by 2
  let config: MapConfig = map_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>)
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

  // Wait a bit for config to be processed
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(15i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  // Collect up to 3 items with timeout
  for _ in 0..3 {
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

  // Verify results
  assert_eq!(results.len(), 3);
  assert!(results.contains(&10)); // 5 * 2
  assert!(results.contains(&20)); // 10 * 2
  assert!(results.contains(&30)); // 15 * 2
}

#[tokio::test]
async fn test_map_node_error_on_no_config() {
  let node = MapNode::new("test_map".to_string());

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
async fn test_map_node_error_on_invalid_type() {
  let node = MapNode::new("test_map".to_string());

  // Create a config that only accepts i32
  let config: MapConfig = map_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>)
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

  // Wait a bit for config to be processed
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

#[tokio::test]
async fn test_map_node_config_update() {
  let node = MapNode::new("test_map".to_string());

  // Create first config (multiply by 2)
  let config1: MapConfig = map_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Create second config (multiply by 3)
  let config2: MapConfig = map_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(Arc::new(*arc_i32 * 3) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send first configuration
  let _ = config_tx
    .send(Arc::new(config1) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data with first config
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Update configuration
  let _ = config_tx
    .send(Arc::new(config2) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data with second config
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  // Collect up to 2 items
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

  // Verify results: first should be 10 (5*2), second should be 15 (5*3)
  assert_eq!(results.len(), 2);
  assert!(results.contains(&10));
  assert!(results.contains(&15));
}
