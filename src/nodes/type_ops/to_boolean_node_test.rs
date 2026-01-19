//! Tests for ToBooleanNode

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::type_ops::ToBooleanNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams() -> (TestSender, TestSender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, inputs)
}

#[tokio::test]
async fn test_to_boolean_node_creation() {
  let node = ToBooleanNode::new("test_to_boolean".to_string());
  assert_eq!(node.name(), "test_to_boolean");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_to_boolean_bool_true() {
  let node = ToBooleanNode::new("test_to_boolean".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send true
  let _ = in_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
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
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(bool_val) = results[0].clone().downcast::<bool>() {
    assert!(*bool_val);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_to_boolean_bool_false() {
  let node = ToBooleanNode::new("test_to_boolean".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send false
  let _ = in_tx
    .send(Arc::new(false) as Arc<dyn Any + Send + Sync>)
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
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(bool_val) = results[0].clone().downcast::<bool>() {
    assert!(!*bool_val);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_to_boolean_i32_zero() {
  let node = ToBooleanNode::new("test_to_boolean".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send 0
  let _ = in_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
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
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(bool_val) = results[0].clone().downcast::<bool>() {
    assert!(!*bool_val);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_to_boolean_i32_non_zero() {
  let node = ToBooleanNode::new("test_to_boolean".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-zero
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
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
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(bool_val) = results[0].clone().downcast::<bool>() {
    assert!(*bool_val);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_to_boolean_multiple_types() {
  let node = ToBooleanNode::new("test_to_boolean".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send multiple items of different types
  let _ = in_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
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

  // First item: bool true -> true
  if let Ok(bool_val) = results[0].clone().downcast::<bool>() {
    assert!(*bool_val);
  } else {
    panic!("First result is not a bool");
  }

  // Second item: i32 0 -> false
  if let Ok(bool_val) = results[1].clone().downcast::<bool>() {
    assert!(!*bool_val);
  } else {
    panic!("Second result is not a bool");
  }

  // Third item: String "hello" -> true
  if let Ok(bool_val) = results[2].clone().downcast::<bool>() {
    assert!(*bool_val);
  } else {
    panic!("Third result is not a bool");
  }
}
