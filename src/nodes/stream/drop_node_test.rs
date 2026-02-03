//! Tests for DropNode

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::stream::DropNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

type AnySender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels
fn create_input_streams() -> (AnySender, AnySender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, inputs)
}

#[tokio::test]
async fn test_drop_node_creation() {
  let node = DropNode::new("test_drop".to_string());
  assert_eq!(node.name(), "test_drop");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_drop_all_items() {
  let node = DropNode::new("test_drop".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send 5 items
  for i in 1..=5 {
    let _ = in_tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
  }
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have dropped all items, so output should be empty
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_drop_empty_stream() {
  let node = DropNode::new("test_drop".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Don't send any items, just close the stream
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have no items
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_drop_many_items() {
  let node = DropNode::new("test_drop".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send many items
  for i in 1..=100 {
    let _ = in_tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
  }
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have dropped all items
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_drop_different_types() {
  let node = DropNode::new("test_drop".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send items of different types
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have dropped all items regardless of type
  assert_eq!(results.len(), 0);
}
