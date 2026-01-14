//! Tests for BreakNode

use crate::graph::node::InputStreams;
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
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "signal".to_string(),
    Box::pin(ReceiverStream::new(signal_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, signal_tx, inputs)
}

#[tokio::test]
async fn test_break_node_creation() {
  use crate::graph::nodes::advanced::BreakNode;
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
  use crate::graph::nodes::advanced::BreakNode;
  let node = BreakNode::new("test_break".to_string());

  let (_config_tx, in_tx, signal_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send items
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  // Send break signal
  let _ = signal_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  // Send more items (should be dropped)
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(signal_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
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

  // Should only have first 2 items (before break signal)
  assert_eq!(results.len(), 2);
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
}

#[tokio::test]
async fn test_break_no_signal() {
  use crate::graph::nodes::advanced::BreakNode;
  let node = BreakNode::new("test_break".to_string());

  let (_config_tx, in_tx, signal_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

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

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
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
  use crate::graph::nodes::advanced::BreakNode;
  let node = BreakNode::new("test_break".to_string());

  let (_config_tx, in_tx, signal_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send break signal first
  let _ = signal_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  // Send items (should all be dropped)
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(signal_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
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

  // Should have no items (break signal received before any items)
  assert_eq!(results.len(), 0);
}
