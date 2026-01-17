//! Tests for ZipNode

use crate::node::InputStreams;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Helper to create input streams from channels for ZipNode
fn create_zip_input_streams(
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
async fn test_zip_node_creation() {
  let node = ZipNode::new("test_zip".to_string(), 2);
  assert_eq!(node.name(), "test_zip");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in_0"));
  assert!(node.has_input_port("in_1"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_zip_two_streams() {
  let node = ZipNode::new("test_zip".to_string(), 2);

  let (_config_tx, mut input_txs, inputs) = create_zip_input_streams(2);
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send items to first stream
  let _ = input_txs[0]
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[0]
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[0]
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send items to second stream
  let _ = input_txs[1]
    .send(Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[1]
    .send(Arc::new("b".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[1]
    .send(Arc::new("c".to_string()) as Arc<dyn Any + Send + Sync>)
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

  assert_eq!(results.len(), 3);

  // Verify first zipped item: [1, "a"]
  if let Ok(zipped_array) = results[0]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(zipped_array.len(), 2);
    if let Ok(val1) = zipped_array[0].clone().downcast::<i32>() {
      assert_eq!(*val1, 1);
    } else {
      panic!("First item is not i32");
    }
    if let Ok(val2) = zipped_array[1].clone().downcast::<String>() {
      assert_eq!(*val2, "a");
    } else {
      panic!("Second item is not String");
    }
  } else {
    panic!("Result is not a Vec");
  }

  // Verify second zipped item: [2, "b"]
  if let Ok(zipped_array) = results[1]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(zipped_array.len(), 2);
    if let Ok(val1) = zipped_array[0].clone().downcast::<i32>() {
      assert_eq!(*val1, 2);
    }
    if let Ok(val2) = zipped_array[1].clone().downcast::<String>() {
      assert_eq!(*val2, "b");
    }
  }

  // Verify third zipped item: [3, "c"]
  if let Ok(zipped_array) = results[2]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(zipped_array.len(), 2);
    if let Ok(val1) = zipped_array[0].clone().downcast::<i32>() {
      assert_eq!(*val1, 3);
    }
    if let Ok(val2) = zipped_array[1].clone().downcast::<String>() {
      assert_eq!(*val2, "c");
    }
  }
}

#[tokio::test]
async fn test_zip_three_streams() {
  let node = ZipNode::new("test_zip".to_string(), 3);

  let (_config_tx, mut input_txs, inputs) = create_zip_input_streams(3);
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send items to each stream
  for i in 0..3 {
    let _ = input_txs[i]
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
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);

  // Verify zipped item: [0, 1, 2]
  if let Ok(zipped_array) = results[0]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(zipped_array.len(), 3);
    for (i, item) in zipped_array.iter().enumerate() {
      if let Ok(val) = item.clone().downcast::<i32>() {
        assert_eq!(*val, i as i32);
      } else {
        panic!("Item {} is not i32", i);
      }
    }
  } else {
    panic!("Result is not a Vec");
  }
}

#[tokio::test]
async fn test_zip_different_lengths() {
  let node = ZipNode::new("test_zip".to_string(), 2);

  let (_config_tx, mut input_txs, inputs) = create_zip_input_streams(2);
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send 3 items to first stream
  for i in 1..=3 {
    let _ = input_txs[0]
      .send(Arc::new(i) as Arc<dyn Any + Send + Sync>)
      .await;
  }

  // Send only 2 items to second stream (shorter)
  let _ = input_txs[1]
    .send(Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_txs[1]
    .send(Arc::new("b".to_string()) as Arc<dyn Any + Send + Sync>)
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

  // Should only have 2 zipped items (shortest stream determines length)
  assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_zip_empty_streams() {
  let node = ZipNode::new("test_zip".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_zip_input_streams(2);
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

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

  // Should have no zipped items
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_zip_one_empty_stream() {
  let node = ZipNode::new("test_zip".to_string(), 2);

  let (_config_tx, input_txs, inputs) = create_zip_input_streams(2);
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

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

  // Should have no zipped items (one stream is empty)
  assert_eq!(results.len(), 0);
}
