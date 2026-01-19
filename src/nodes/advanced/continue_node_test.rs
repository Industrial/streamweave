//! Tests for ContinueNode
#![allow(unused)]
#![allow(clippy::type_complexity)]

use crate::node::{InputStreams, Node, OutputStreams};
use crate::nodes::advanced::ContinueNode;
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
async fn test_continue_node_creation() {
  let node = ContinueNode::new("test_continue".to_string());
  assert_eq!(node.name(), "test_continue");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("signal"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_continue_basic() {
  let node = ContinueNode::new("test_continue".to_string());

  let (_config_tx, in_tx, signal_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send items
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send continue signal
  let _ = signal_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send next item (should be skipped)
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send another item (should be forwarded)
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

  // Should have items 1 and 3 (item 2 was skipped)
  assert_eq!(results.len(), 2);
  if let Ok(num_val) = results[0].clone().downcast::<i32>() {
    assert_eq!(*num_val, 1);
  } else {
    panic!("First result is not i32");
  }
  if let Ok(num_val) = results[1].clone().downcast::<i32>() {
    assert_eq!(*num_val, 3);
  } else {
    panic!("Second result is not i32");
  }
}

#[tokio::test]
async fn test_continue_no_signal() {
  let node = ContinueNode::new("test_continue".to_string());

  let (_config_tx, in_tx, signal_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send items without continue signal
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

  // Should have all items (no continue signal)
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
async fn test_continue_multiple_signals() {
  let node = ContinueNode::new("test_continue".to_string());

  let (_config_tx, in_tx, signal_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send items with multiple continue signals
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send continue signal
  let _ = signal_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send next item (should be skipped)
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send another continue signal
  let _ = signal_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send next item (should be skipped)
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send another item (should be forwarded)
  let _ = in_tx
    .send(Arc::new(4i32) as Arc<dyn Any + Send + Sync>)
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

  // Should have items 1 and 4 (items 2 and 3 were skipped)
  assert_eq!(results.len(), 2);
  if let Ok(num_val) = results[0].clone().downcast::<i32>() {
    assert_eq!(*num_val, 1);
  } else {
    panic!("First result is not i32");
  }
  if let Ok(num_val) = results[1].clone().downcast::<i32>() {
    assert_eq!(*num_val, 4);
  } else {
    panic!("Second result is not i32");
  }
}

#[tokio::test]
async fn test_continue_signal_before_item() {
  let node = ContinueNode::new("test_continue".to_string());

  let (_config_tx, in_tx, signal_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send continue signal first
  let _ = signal_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send item (should be skipped)
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send another item (should be forwarded)
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
          if results.len() == 1 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have only item 2 (item 1 was skipped)
  assert_eq!(results.len(), 1);
  if let Ok(num_val) = results[0].clone().downcast::<i32>() {
    assert_eq!(*num_val, 2);
  } else {
    panic!("Result is not i32");
  }
}
