//! Tests for InterleaveNode

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::stream::InterleaveNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels for InterleaveNode
fn create_interleave_input_streams(
  num_inputs: usize,
) -> (
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  Vec<mpsc::Sender<Arc<dyn Any + Send + Sync>>>,
  InputStreams,
) {
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
async fn test_interleave_node_creation() {
  let node = InterleaveNode::new("test_interleave".to_string(), 2);
  assert_eq!(node.name(), "test_interleave");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in_0"));
  assert!(node.has_input_port("in_1"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_interleave_two_streams() {
  let node = InterleaveNode::new("test_interleave".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_interleave_input_streams(2);
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send items to first stream
  let _ = input_txs[0]
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[0]
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[0]
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send items to second stream
  let _ = input_txs[1]
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[1]
    .send(Arc::new(4i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[1]
    .send(Arc::new(6i32) as Arc<dyn Any + Send + Sync>)
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
          if results.len() == 6 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 6);

  // Verify interleaved order: 1, 2, 3, 4, 5, 6
  for (idx, result) in results.iter().enumerate() {
    if let Ok(val) = result.clone().downcast::<i32>() {
      assert_eq!(*val, (idx + 1) as i32);
    } else {
      panic!("Result {} is not i32", idx);
    }
  }
}

#[tokio::test]
async fn test_interleave_three_streams() {
  let node = InterleaveNode::new("test_interleave".to_string(), 3);

  let (_config_tx, input_txs, inputs) = create_interleave_input_streams(3);
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send items to each stream
  for i in 0..3 {
    let _ = input_txs[i]
      .send(Arc::new(i as i32) as Arc<dyn Any + Send + Sync>)
      .await;
    let _ = input_txs[i]
      .send(Arc::new((i + 3) as i32) as Arc<dyn Any + Send + Sync>)
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
          if results.len() == 6 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 6);

  // Verify interleaved order: 0, 1, 2, 3, 4, 5
  for (idx, result) in results.iter().enumerate() {
    if let Ok(val) = result.clone().downcast::<i32>() {
      assert_eq!(*val, idx as i32);
    } else {
      panic!("Result {} is not i32", idx);
    }
  }
}

#[tokio::test]
async fn test_interleave_different_lengths() {
  let node = InterleaveNode::new("test_interleave".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_interleave_input_streams(2);
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

  // Should have 5 items: 1, 10, 2, 20, 3 (interleaved, then remaining from first stream)
  assert_eq!(results.len(), 5);

  // Verify interleaved order: 1, 10, 2, 20, 3
  let expected = [1, 10, 2, 20, 3];
  for (idx, result) in results.iter().enumerate() {
    if let Ok(val) = result.clone().downcast::<i32>() {
      assert_eq!(*val, expected[idx]);
    } else {
      panic!("Result {} is not i32", idx);
    }
  }
}

#[tokio::test]
async fn test_interleave_empty_streams() {
  let node = InterleaveNode::new("test_interleave".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_interleave_input_streams(2);
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

  // Should have no interleaved items
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_interleave_one_empty_stream() {
  let node = InterleaveNode::new("test_interleave".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_interleave_input_streams(2);
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

  // Should have 2 items from first stream (second stream was empty, so skipped)
  assert_eq!(results.len(), 2);
  if let (Ok(val1), Ok(val2)) = (
    results[0].clone().downcast::<i32>(),
    results[1].clone().downcast::<i32>(),
  ) {
    assert_eq!(*val1, 1);
    assert_eq!(*val2, 2);
  } else {
    panic!("Results are not i32");
  }
}
