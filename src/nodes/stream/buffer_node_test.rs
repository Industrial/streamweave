#![allow(unused_imports, dead_code, unused, clippy::type_complexity)]

use super::BufferNode;
use crate::node::{InputStreams, Node, OutputStreams};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams() -> (
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (size_tx, size_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "size".to_string(),
    Box::pin(ReceiverStream::new(size_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, size_tx, inputs)
}

#[tokio::test]
async fn test_buffer_node_creation() {
  let node = BufferNode::new("test_buffer".to_string());
  assert_eq!(node.name(), "test_buffer");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("size"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_buffer_batches() {
  let node = BufferNode::new("test_buffer".to_string());

  let (_config_tx, in_tx, size_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send buffer size: 3 items per batch
  let buffer_size = 3usize;
  size_tx
    .send(Arc::new(buffer_size) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send 7 items: should produce 2 full batches (3+3) and discard the last item
  let test_values = vec![1i32, 2i32, 3i32, 4i32, 5i32, 6i32, 7i32];
  for value in test_values {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream
  drop(size_tx); // Close the size stream

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

  // Should have 2 batches
  assert_eq!(results.len(), 2);

  // Check first batch: [1, 2, 3]
  if let Ok(batch1) = results[0]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(batch1.len(), 3);
    for (i, item) in batch1.iter().enumerate() {
      if let Ok(value) = item.clone().downcast::<i32>() {
        assert_eq!(*value, (i + 1) as i32);
      } else {
        panic!("Item {} in batch 1 is not an i32", i);
      }
    }
  } else {
    panic!("First result is not a Vec");
  }

  // Check second batch: [4, 5, 6]
  if let Ok(batch2) = results[1]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(batch2.len(), 3);
    for (i, item) in batch2.iter().enumerate() {
      if let Ok(value) = item.clone().downcast::<i32>() {
        assert_eq!(*value, (i + 4) as i32);
      } else {
        panic!("Item {} in batch 2 is not an i32", i);
      }
    }
  } else {
    panic!("Second result is not a Vec");
  }
}

#[tokio::test]
async fn test_buffer_empty_stream() {
  let node = BufferNode::new("test_buffer".to_string());

  let (_config_tx, in_tx, size_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send buffer size
  let buffer_size = 2usize;
  size_tx
    .send(Arc::new(buffer_size) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send no items: empty stream → no batches emitted
  drop(in_tx); // Close the input stream immediately
  drop(size_tx); // Close the size stream

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

  // Should have no batches (empty stream)
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_buffer_partial_batch_discarded() {
  let node = BufferNode::new("test_buffer".to_string());

  let (_config_tx, in_tx, size_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send buffer size: 3 items per batch
  let buffer_size = 3usize;
  size_tx
    .send(Arc::new(buffer_size) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send 4 items: should produce 1 full batch and discard the partial batch
  let test_values = vec![1i32, 2i32, 3i32, 4i32];
  for value in test_values {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream
  drop(size_tx); // Close the size stream

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

  // Should have 1 batch (partial batch discarded)
  assert_eq!(results.len(), 1);

  // Check batch: [1, 2, 3]
  if let Ok(batch) = results[0]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(batch.len(), 3);
    for (i, item) in batch.iter().enumerate() {
      if let Ok(value) = item.clone().downcast::<i32>() {
        assert_eq!(*value, (i + 1) as i32);
      } else {
        panic!("Item {} in batch is not an i32", i);
      }
    }
  } else {
    panic!("Result is not a Vec");
  }
}
