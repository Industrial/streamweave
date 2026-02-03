//! Tests for MergeNode

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::stream::MergeNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

type AnySender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels for MergeNode
fn create_merge_input_streams(num_inputs: usize) -> (AnySender, Vec<AnySender>, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let mut input_txs = Vec::new();
  let mut inputs = HashMap::new();

  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );

  for i in 0..num_inputs {
    let (in_tx, in_rx) = mpsc::channel(10);
    input_txs.push(in_tx);
    inputs.insert(
      format!("in_{}", i),
      Box::pin(ReceiverStream::new(in_rx)) as crate::node::InputStream,
    );
  }

  (config_tx, input_txs, inputs)
}

#[tokio::test]
async fn test_merge_node_creation() {
  let node = MergeNode::new("test_merge".to_string(), 2);
  assert_eq!(node.name(), "test_merge");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in_0"));
  assert!(node.has_input_port("in_1"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_merge_two_streams() {
  let node = MergeNode::new("test_merge".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_merge_input_streams(2);
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send items to first stream
  let _ = input_txs[0]
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[0]
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send items to second stream
  let _ = input_txs[1]
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[1]
    .send(Arc::new(4i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close all streams
  for tx in input_txs {
    drop(tx);
  }

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
          if results.len() == 4 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have 4 merged items (order may vary)
  assert_eq!(results.len(), 4);

  // Verify all items are present (order is non-deterministic)
  let mut values: Vec<i32> = results
    .iter()
    .filter_map(|r| r.clone().downcast::<i32>().ok().map(|arc| *arc))
    .collect();
  values.sort();
  assert_eq!(values, vec![1, 2, 3, 4]);
}

#[tokio::test]
async fn test_merge_three_streams() {
  let node = MergeNode::new("test_merge".to_string(), 3);

  let (_config_tx, input_txs, inputs) = create_merge_input_streams(3);
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send items to each stream
  for (i, tx) in input_txs.iter().enumerate().take(3) {
    let _ = tx
      .send(Arc::new(i as i32) as Arc<dyn Any + Send + Sync>)
      .await;
  }

  // Close all streams
  for tx in input_txs {
    drop(tx);
  }

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
          if results.len() == 3 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have 3 merged items
  assert_eq!(results.len(), 3);

  // Verify all items are present (order is non-deterministic)
  let mut values: Vec<i32> = results
    .iter()
    .filter_map(|r| r.clone().downcast::<i32>().ok().map(|arc| *arc))
    .collect();
  values.sort();
  assert_eq!(values, vec![0, 1, 2]);
}

#[tokio::test]
async fn test_merge_different_lengths() {
  let node = MergeNode::new("test_merge".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_merge_input_streams(2);
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send 3 items to first stream
  for i in 1..=3 {
    let _ = input_txs[0]
      .send(Arc::new(i) as Arc<dyn Any + Send + Sync>)
      .await;
  }

  // Send only 2 items to second stream (shorter)
  let _ = input_txs[1]
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[1]
    .send(Arc::new(20i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close all streams
  for tx in input_txs {
    drop(tx);
  }

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

  // Should have 5 merged items (all items from both streams)
  assert_eq!(results.len(), 5);

  // Verify all items are present (order is non-deterministic)
  let mut values: Vec<i32> = results
    .iter()
    .filter_map(|r| r.clone().downcast::<i32>().ok().map(|arc| *arc))
    .collect();
  values.sort();
  assert_eq!(values, vec![1, 2, 3, 10, 20]);
}

#[tokio::test]
async fn test_merge_empty_streams() {
  let node = MergeNode::new("test_merge".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_merge_input_streams(2);
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Don't send any items, just close streams
  for tx in input_txs {
    drop(tx);
  }

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

  // Should have no merged items
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_merge_one_empty_stream() {
  let node = MergeNode::new("test_merge".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_merge_input_streams(2);
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send items to first stream
  let _ = input_txs[0]
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[0]
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Don't send anything to second stream (empty)
  // Drop all senders by dropping the Vec
  drop(input_txs);

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

  // Should have 2 items from first stream
  assert_eq!(results.len(), 2);
  let mut values: Vec<i32> = results
    .iter()
    .filter_map(|r| r.clone().downcast::<i32>().ok().map(|arc| *arc))
    .collect();
  values.sort();
  assert_eq!(values, vec![1, 2]);
}

#[tokio::test]
async fn test_merge_different_types() {
  let node = MergeNode::new("test_merge".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_merge_input_streams(2);
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send different types to each stream
  let _ = input_txs[0]
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[1]
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close all streams
  for tx in input_txs {
    drop(tx);
  }

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
          if results.len() == 2 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have 2 merged items
  assert_eq!(results.len(), 2);

  // Verify both types are present
  let has_i32 = results.iter().any(|r| r.clone().downcast::<i32>().is_ok());
  let has_string = results
    .iter()
    .any(|r| r.clone().downcast::<String>().is_ok());
  assert!(has_i32);
  assert!(has_string);
}
