//! Tests for SyncNode

use crate::node::{InputStreams, Node};
use crate::nodes::common::{TestSender, TestSenderVec};
use crate::nodes::sync_node::{SyncConfig, SyncNode};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams(num_inputs: usize) -> (TestSender, TestSenderVec, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let mut input_txs = Vec::new();
  let mut inputs = HashMap::new();

  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );

  for i in 0..num_inputs {
    let (tx, rx) = mpsc::channel(10);
    input_txs.push(tx);
    inputs.insert(
      format!("in_{}", i),
      Box::pin(ReceiverStream::new(rx)) as crate::node::InputStream,
    );
  }

  (config_tx, input_txs, inputs)
}

#[tokio::test]
async fn test_sync_node_creation() {
  let node = SyncNode::new("test_sync".to_string(), 3);
  assert_eq!(node.name(), "test_sync");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in_0"));
  assert!(node.has_input_port("in_1"));
  assert!(node.has_input_port("in_2"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
  assert!(!node.has_config());
}

#[tokio::test]
async fn test_sync_node_basic_sync() {
  let node = SyncNode::new("test_sync".to_string(), 2);

  let (config_tx, input_txs, inputs) = create_input_streams(2);
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let config = Arc::new(SyncConfig {
    num_inputs: 2,
    timeout: None,
  });
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Send values to both inputs
  let _ = input_txs[0]
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[1]
    .send(Arc::new(20i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_vec) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
            results.push(arc_vec);
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  let combined = &results[0];
  assert_eq!(combined.len(), 2);
  assert_eq!(*combined[0].clone().downcast::<i32>().unwrap(), 10);
  assert_eq!(*combined[1].clone().downcast::<i32>().unwrap(), 20);
}

#[tokio::test]
async fn test_sync_node_three_inputs() {
  let node = SyncNode::new("test_sync".to_string(), 3);

  let (config_tx, input_txs, inputs) = create_input_streams(3);
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let config = Arc::new(SyncConfig {
    num_inputs: 3,
    timeout: None,
  });
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Send values to all three inputs
  let _ = input_txs[0]
    .send(Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[1]
    .send(Arc::new("b".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[2]
    .send(Arc::new("c".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_vec) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
            results.push(arc_vec);
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  let combined = &results[0];
  assert_eq!(combined.len(), 3);
  assert_eq!(&*combined[0].clone().downcast::<String>().unwrap(), "a");
  assert_eq!(&*combined[1].clone().downcast::<String>().unwrap(), "b");
  assert_eq!(&*combined[2].clone().downcast::<String>().unwrap(), "c");
}

#[tokio::test]
async fn test_sync_node_error_no_config() {
  let node = SyncNode::new("test_sync".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_input_streams(2);
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values without configuration
  let _ = input_txs[0]
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[1]
    .send(Arc::new(20i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Wait a bit - node should be waiting for config
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  // No output should be produced (node is waiting for config)
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(50));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(_item) = result {
          results.push(());
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have no output (waiting for config)
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_sync_node_timeout() {
  let node = SyncNode::new("test_sync".to_string(), 2);

  let (config_tx, input_txs, inputs) = create_input_streams(2);
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration with short timeout
  let config = Arc::new(SyncConfig {
    num_inputs: 2,
    timeout: Some(Duration::from_millis(50)),
  });
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send only one value (not both)
  let _ = input_txs[0]
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Wait for timeout
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  // Collect error
  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_string) = item.downcast::<String>() {
            errors.push(arc_string.clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("timeout"));
}
