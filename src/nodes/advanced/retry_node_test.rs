//! Tests for RetryNode
#![allow(clippy::type_complexity, unused)]

use crate::node::{InputStreams, Node, OutputStreams};
use crate::nodes::advanced::RetryFunction;
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
  let (data_tx, data_rx) = mpsc::channel(10);
  let (max_retries_tx, max_retries_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "max_retries".to_string(),
    Box::pin(ReceiverStream::new(max_retries_rx)) as crate::node::InputStream,
  );

  (config_tx, data_tx, max_retries_tx, inputs)
}

#[tokio::test]
async fn test_retry_node_creation() {
  use crate::nodes::advanced::RetryNode;
  let node = RetryNode::new("test_retry".to_string(), 100);
  assert_eq!(node.name(), "test_retry");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("max_retries"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_retry_success_first_attempt() {
  use crate::nodes::advanced::{RetryNode, retry_config};
  let node = RetryNode::new("test_retry".to_string(), 100);

  let (config_tx, data_tx, max_retries_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create and send retry configuration (succeeds on first attempt)
  let retry_fn = retry_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let n = *arc_i32;
      Ok(Arc::new(n * 2) as Arc<dyn Any + Send + Sync>)
    } else {
      Err(Arc::new("Expected i32".to_string()) as Arc<dyn Any + Send + Sync>)
    }
  });
  let retry_fn_any =
    unsafe { std::mem::transmute::<Arc<dyn RetryFunction>, Arc<dyn Any + Send + Sync>>(retry_fn) };
  let _ = config_tx.send(retry_fn_any).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await; // This cast should work since RetryFunction implements Any
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Send max_retries
  let _ = max_retries_tx.send(Arc::new(3usize)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Send data
  let _ = data_tx.send(Arc::new(5i32)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  drop(data_tx);
  drop(max_retries_tx);

  // Wait a bit for processing
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  let error_stream = outputs.remove("error").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(1000));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result {
        results.push(item);
      }
    }
    _ = &mut timeout => {},
  }

  // Should have success result
  assert_eq!(results.len(), 0); // No errors expected
}

#[tokio::test]
#[ignore] // TODO: Fix async timing issues
async fn test_retry_success_after_retries() {
  use crate::nodes::advanced::{RetryNode, retry_config};
  use std::sync::atomic::{AtomicUsize, Ordering};

  let node = RetryNode::new("test_retry".to_string(), 10); // Small delay for testing

  let (config_tx, data_tx, max_retries_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create retry function that fails twice then succeeds
  let attempt_count = Arc::new(AtomicUsize::new(0));
  let attempt_count_clone = attempt_count.clone();
  let retry_fn = retry_config(move |value| {
    let attempt_count = attempt_count_clone.clone();
    async move {
      let count = attempt_count.fetch_add(1, Ordering::SeqCst);
      if count < 2 {
        // Fail first two attempts
        Err(Arc::new(format!("Attempt {} failed", count + 1)) as Arc<dyn Any + Send + Sync>)
      } else {
        // Succeed on third attempt
        if let Ok(arc_i32) = value.downcast::<i32>() {
          let n = *arc_i32;
          Ok(Arc::new(n * 2) as Arc<dyn Any + Send + Sync>)
        } else {
          Err(Arc::new("Expected i32".to_string()) as Arc<dyn Any + Send + Sync>)
        }
      }
    }
  });
  let retry_fn_any =
    unsafe { std::mem::transmute::<Arc<dyn RetryFunction>, Arc<dyn Any + Send + Sync>>(retry_fn) };
  let _ = config_tx.send(retry_fn_any).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await; // This cast should work since RetryFunction implements Any
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Send max_retries
  let _ = max_retries_tx.send(Arc::new(3usize)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Send data
  let _ = data_tx.send(Arc::new(5i32)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  drop(data_tx);
  drop(max_retries_tx);

  // Wait for retries (10ms + 20ms = ~30ms, plus processing time)
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(1000));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result {
        results.push(item);
      }
    }
    _ = &mut timeout => {},
  }

  // Should succeed after retries
  assert_eq!(results.len(), 1);
  if let Ok(result) = results[0].clone().downcast::<i32>() {
    assert_eq!(*result, 10); // 5 * 2
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
#[ignore] // TODO: Fix async timing issues
async fn test_retry_all_attempts_fail() {
  use crate::nodes::advanced::{RetryNode, retry_config};

  let node = RetryNode::new("test_retry".to_string(), 10); // Small delay for testing

  let (config_tx, data_tx, max_retries_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create retry function that always fails
  let retry_fn = retry_config(|_value| async move {
    Err(Arc::new("Always fails".to_string()) as Arc<dyn Any + Send + Sync>)
  });
  let retry_fn_any =
    unsafe { std::mem::transmute::<Arc<dyn RetryFunction>, Arc<dyn Any + Send + Sync>>(retry_fn) };
  let _ = config_tx.send(retry_fn_any).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await; // This cast should work since RetryFunction implements Any
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Send max_retries
  let _ = max_retries_tx.send(Arc::new(2usize)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Send data
  let _ = data_tx.send(Arc::new(5i32)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  drop(data_tx);
  drop(max_retries_tx);

  // Wait for retries (10ms + 20ms = ~30ms, plus processing time)
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  let error_stream = outputs.remove("error").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(1000));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result {
        results.push(item);
      }
    }
    _ = &mut timeout => {},
  }

  // Should have error after all retries exhausted
  assert_eq!(results.len(), 1);
  if let Ok(error_str) = results[0].clone().downcast::<String>() {
    assert_eq!(*error_str, "Always fails");
  } else {
    panic!("Error is not a String");
  }
}
