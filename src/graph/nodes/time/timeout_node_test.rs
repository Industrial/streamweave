//! Tests for TimeoutNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::time::TimeoutNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
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
  let (timeout_tx, timeout_rx) = mpsc::channel(10);

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
    "timeout".to_string(),
    Box::pin(ReceiverStream::new(timeout_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, timeout_tx, inputs)
}

#[tokio::test]
async fn test_timeout_node_creation() {
  let node = TimeoutNode::new("test_timeout".to_string());
  assert_eq!(node.name(), "test_timeout");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("timeout"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_timeout_item_arrives_in_time() {
  let node = TimeoutNode::new("test_timeout".to_string());

  let (_config_tx, in_tx, timeout_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send timeout: 100 milliseconds
  let _ = timeout_tx
    .send(Arc::new(100i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send item immediately (should arrive within timeout)
  tokio::time::sleep(Duration::from_millis(10)).await; // Small delay to ensure timeout is set
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(timeout_tx);

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
  if let Ok(value) = results[0].clone().downcast::<i32>() {
    assert_eq!(*value, 42i32);
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
async fn test_timeout_expires() {
  let node = TimeoutNode::new("test_timeout".to_string());

  let (_config_tx, in_tx, timeout_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send timeout: 50 milliseconds (short timeout)
  let _ = timeout_tx
    .send(Arc::new(50i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(timeout_tx);

  // Don't send any items - wait for timeout
  tokio::time::sleep(Duration::from_millis(10)).await; // Small delay to ensure timeout is set
  drop(in_tx); // Close input stream without sending items

  // Check error output
  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<String> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            errors.push((*arc_str).clone());
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("Timeout"));
}

#[tokio::test]
async fn test_timeout_multiple_items() {
  let node = TimeoutNode::new("test_timeout".to_string());

  let (_config_tx, in_tx, timeout_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send timeout: 100 milliseconds
  let _ = timeout_tx
    .send(Arc::new(100i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(timeout_tx);

  // Send multiple items quickly (all should arrive within timeout)
  tokio::time::sleep(Duration::from_millis(10)).await; // Small delay to ensure timeout is set
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
  // Verify order is preserved
  if let (Ok(val1), Ok(val2), Ok(val3)) = (
    results[0].clone().downcast::<i32>(),
    results[1].clone().downcast::<i32>(),
    results[2].clone().downcast::<i32>(),
  ) {
    assert_eq!(*val1, 1i32);
    assert_eq!(*val2, 2i32);
    assert_eq!(*val3, 3i32);
  } else {
    panic!("Results are not i32");
  }
}

#[tokio::test]
async fn test_timeout_invalid_duration() {
  let node = TimeoutNode::new("test_timeout".to_string());

  let (_config_tx, in_tx, timeout_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid timeout: negative value
  let _ = timeout_tx
    .send(Arc::new(-10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(timeout_tx);
  drop(in_tx);

  // Check error output
  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<String> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            errors.push((*arc_str).clone());
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("negative"));
}

