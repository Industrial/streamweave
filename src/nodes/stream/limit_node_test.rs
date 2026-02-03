//! Tests for LimitNode

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::stream::LimitNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

type AnySender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels
fn create_input_streams() -> (AnySender, AnySender, AnySender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (max_size_tx, max_size_rx) = mpsc::channel(10);

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
    "max_size".to_string(),
    Box::pin(ReceiverStream::new(max_size_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, max_size_tx, inputs)
}

#[tokio::test]
async fn test_limit_node_creation() {
  let node = LimitNode::new("test_limit".to_string());
  assert_eq!(node.name(), "test_limit");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("max_size"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_limit_stream_size() {
  let node = LimitNode::new("test_limit".to_string());

  let (_config_tx, in_tx, max_size_tx, inputs) = create_input_streams();
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send max_size: 3
  let _ = max_size_tx
    .send(Arc::new(3usize) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send 5 items
  for i in 1..=5 {
    let _ = in_tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
  }
  drop(in_tx);
  drop(max_size_tx);

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
  // Verify we got the first 3 items
  if let (Ok(val1), Ok(val2), Ok(val3)) = (
    results[0].clone().downcast::<i32>(),
    results[1].clone().downcast::<i32>(),
    results[2].clone().downcast::<i32>(),
  ) {
    assert_eq!(*val1, 1);
    assert_eq!(*val2, 2);
    assert_eq!(*val3, 3);
  } else {
    panic!("Results are not i32");
  }
}

#[tokio::test]
async fn test_limit_zero_size() {
  let node = LimitNode::new("test_limit".to_string());

  let (_config_tx, in_tx, max_size_tx, inputs) = create_input_streams();
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send max_size: 0
  let _ = max_size_tx
    .send(Arc::new(0usize) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send 3 items
  for i in 1..=3 {
    let _ = in_tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
  }
  drop(in_tx);
  drop(max_size_tx);

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

  // Should have limited to 0 items
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_limit_more_than_available() {
  let node = LimitNode::new("test_limit".to_string());

  let (_config_tx, in_tx, max_size_tx, inputs) = create_input_streams();
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send max_size: 10
  let _ = max_size_tx
    .send(Arc::new(10usize) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send only 3 items
  for i in 1..=3 {
    let _ = in_tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
  }
  drop(in_tx);
  drop(max_size_tx);

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

  // Should have forwarded all 3 available items
  assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_limit_i32_max_size() {
  let node = LimitNode::new("test_limit".to_string());

  let (_config_tx, in_tx, max_size_tx, inputs) = create_input_streams();
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send max_size: 2 as i32
  let _ = max_size_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send 5 items
  for i in 1..=5 {
    let _ = in_tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
  }
  drop(in_tx);
  drop(max_size_tx);

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

  assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_limit_invalid_max_size_negative() {
  let node = LimitNode::new("test_limit".to_string());

  let (_config_tx, in_tx, max_size_tx, inputs) = create_input_streams();
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send invalid max_size: negative value
  let _ = max_size_tx
    .send(Arc::new(-5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(max_size_tx);
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
          if let Ok(arc_str) = Arc::downcast::<String>(item.clone()) {
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

#[tokio::test]
async fn test_limit_invalid_max_size_type() {
  let node = LimitNode::new("test_limit".to_string());

  let (_config_tx, in_tx, max_size_tx, inputs) = create_input_streams();
  let execute_result: Result<OutputStreams, NodeExecutionError> = node.execute(inputs).await;
  let mut outputs = execute_result.unwrap();

  // Send invalid max_size: string instead of numeric
  let _ = max_size_tx
    .send(Arc::new("invalid".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(max_size_tx);
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
          if let Ok(arc_str) = Arc::downcast::<String>(item.clone()) {
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
  assert!(errors[0].contains("numeric") || errors[0].contains("Unsupported"));
}
