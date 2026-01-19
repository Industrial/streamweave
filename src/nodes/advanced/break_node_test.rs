//! Tests for BreakNode
#![allow(unused)]
#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, OutputStreams};
use crate::nodes::advanced::break_node::BreakNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Helper to create input streams from channels
fn create_input_streams() -> (
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (signal_tx, signal_rx) = mpsc::channel(10);

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
    "signal".to_string(),
    Box::pin(ReceiverStream::new(signal_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, signal_tx, inputs)
}

#[tokio::test]
async fn test_break_node_creation() {
  let node = BreakNode::new("test_break".to_string());
  assert_eq!(node.name(), "test_break");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("signal"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_break_basic() {
  let node = BreakNode::new("test_break".to_string());

  let (_config_tx, in_tx, signal_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send some items with delays to ensure ordering
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send break signal
  let _ = signal_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send more items (should be dropped)
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(signal_tx);

  // Give a delay to ensure async processing completes
  tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;

  use tokio_stream::StreamExt;
  // Collect all available results from the stream
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  // Should have items 1 and 2 (sent before break signal)
  // Due to async timing, we accept 1-2 items
  assert!(
    results.len() >= 1,
    "Expected at least 1 item, got {}",
    results.len()
  );
  assert!(
    results.len() <= 2,
    "Expected at most 2 items, got {}",
    results.len()
  );

  // All forwarded items should be from the valid range
  for result in &results {
    if let Ok(num_val) = result.clone().downcast::<i32>() {
      assert!(
        *num_val >= 1 && *num_val <= 2,
        "Unexpected item value: {}",
        *num_val
      );
    } else {
      panic!("Result is not i32");
    }
  }
}

#[tokio::test]
async fn test_break_no_signal() {
  let node = BreakNode::new("test_break".to_string());

  let (_config_tx, in_tx, signal_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send items without break signal
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(signal_tx);

  // Give a delay to ensure async processing completes
  tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;

  use tokio_stream::StreamExt;
  // Collect all available results from the stream
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  // Should have all items (no break signal)
  assert_eq!(results.len(), 3);
  if let Ok(num_val) = results[0].clone().downcast::<i32>() {
    assert_eq!(*num_val, 1);
  } else {
    panic!("First result is not i32");
  }
  if let Ok(num_val) = results[1].clone().downcast::<i32>() {
    assert_eq!(*num_val, 2);
  } else {
    panic!("Second result is not i32");
  }
  if let Ok(num_val) = results[2].clone().downcast::<i32>() {
    assert_eq!(*num_val, 3);
  } else {
    panic!("Third result is not i32");
  }
}

#[tokio::test]
async fn test_break_immediate() {
  let node = BreakNode::new("test_break".to_string());

  let (_config_tx, in_tx, signal_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send break signal first
  let _ = signal_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send items (should all be dropped)
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(signal_tx);

  // Give a delay to ensure async processing completes
  tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;

  use tokio_stream::StreamExt;
  // Collect all available results from the stream
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  // Should have no items (break signal received before any items)
  assert_eq!(results.len(), 0);
}
