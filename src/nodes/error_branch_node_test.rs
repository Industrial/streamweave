//! Tests for ErrorBranchNode

use crate::node::{InputStreams, Node};
use crate::nodes::error_branch_node::ErrorBranchNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams() -> (
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (data_tx, data_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as crate::node::InputStream,
  );

  (config_tx, data_tx, inputs)
}

#[tokio::test]
async fn test_error_branch_node_creation() {
  let node = ErrorBranchNode::new("test_error_branch".to_string());
  assert_eq!(node.name(), "test_error_branch");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("success"));
  assert!(node.has_output_port("error"));
  assert!(!node.has_config());
}

#[tokio::test]
async fn test_error_branch_node_ok_value() {
  let node = ErrorBranchNode::new("test_error_branch".to_string());

  let (_config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send Ok value: Result<Arc<dyn Any + Send + Sync>, String>
  let ok_value: Arc<dyn Any + Send + Sync> = Arc::new(42i32);
  let result: Result<Arc<dyn Any + Send + Sync>, String> = Ok(ok_value);
  let _ = data_tx
    .send(Arc::new(result) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let success_stream = outputs.remove("success").unwrap();
  let mut results = Vec::new();
  let mut stream = success_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            results.push(*arc_i32);
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], 42);
}

#[tokio::test]
async fn test_error_branch_node_err_value() {
  let node = ErrorBranchNode::new("test_error_branch".to_string());

  let (_config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send Err value: Result<Arc<dyn Any + Send + Sync>, String>
  let error_msg = "Something went wrong".to_string();
  let result: Result<Arc<dyn Any + Send + Sync>, String> = Err(error_msg.clone());
  let _ = data_tx
    .send(Arc::new(result) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect error results
  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_string) = item.downcast::<String>() {
            errors.push(arc_string.clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert_eq!(&*errors[0], &error_msg);
}

#[tokio::test]
async fn test_error_branch_node_string_ok() {
  let node = ErrorBranchNode::new("test_error_branch".to_string());

  let (_config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send Ok value with String
  let ok_value: Arc<dyn Any + Send + Sync> = Arc::new("success".to_string());
  let result: Result<Arc<dyn Any + Send + Sync>, String> = Ok(ok_value);
  let _ = data_tx
    .send(Arc::new(result) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let success_stream = outputs.remove("success").unwrap();
  let mut results = Vec::new();
  let mut stream = success_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_string) = item.downcast::<String>() {
            results.push(arc_string.clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  assert_eq!(&*results[0], "success");
}

#[tokio::test]
async fn test_error_branch_node_invalid_type() {
  let node = ErrorBranchNode::new("test_error_branch".to_string());

  let (_config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send a non-Result type
  let _ = data_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect error results
  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_string) = item.downcast::<String>() {
            errors.push(arc_string.clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("Expected Result"));
}

#[tokio::test]
async fn test_error_branch_node_multiple_results() {
  let node = ErrorBranchNode::new("test_error_branch".to_string());

  let (_config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send multiple results: Ok, Err, Ok
  let result1: Result<Arc<dyn Any + Send + Sync>, String> = Ok(Arc::new(1i32));
  let _ = data_tx
    .send(Arc::new(result1) as Arc<dyn Any + Send + Sync>)
    .await;

  let result2: Result<Arc<dyn Any + Send + Sync>, String> = Err("error1".to_string());
  let _ = data_tx
    .send(Arc::new(result2) as Arc<dyn Any + Send + Sync>)
    .await;

  let result3: Result<Arc<dyn Any + Send + Sync>, String> = Ok(Arc::new(3i32));
  let _ = data_tx
    .send(Arc::new(result3) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect success results
  let success_stream = outputs.remove("success").unwrap();
  let mut successes = Vec::new();
  let mut stream = success_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            successes.push(*arc_i32);
            if successes.len() >= 2 {
              break;
            }
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Collect error results
  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_string) = item.downcast::<String>() {
            errors.push(arc_string.clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(successes.len(), 2);
  assert_eq!(successes, vec![1, 3]);
  assert_eq!(errors.len(), 1);
  assert_eq!(&*errors[0], "error1");
}
