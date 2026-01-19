//! Tests for VariableNode

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::variable_node::VariableNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams() -> (TestSender, TestSender, TestSender, TestSender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (read_tx, read_rx) = mpsc::channel(10);
  let (write_tx, write_rx) = mpsc::channel(10);
  let (value_tx, value_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "read".to_string(),
    Box::pin(ReceiverStream::new(read_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "write".to_string(),
    Box::pin(ReceiverStream::new(write_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "value".to_string(),
    Box::pin(ReceiverStream::new(value_rx)) as crate::node::InputStream,
  );

  (config_tx, read_tx, write_tx, value_tx, inputs)
}

#[tokio::test]
async fn test_variable_node_creation() {
  let node = VariableNode::new("test_variable".to_string());
  assert_eq!(node.name(), "test_variable");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("read"));
  assert!(node.has_input_port("write"));
  assert!(node.has_input_port("value"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
  assert!(!node.has_config());
}

#[tokio::test]
async fn test_variable_node_write_and_read() {
  let node = VariableNode::new("test_variable".to_string());

  let (_config_tx, read_tx, write_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Write a variable: send name on "write" port, then value on "value" port
  let var_name = "counter".to_string();
  let var_value = Arc::new(42i32) as Arc<dyn Any + Send + Sync>;

  let _ = write_tx
    .send(Arc::new(var_name.clone()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  let _ = value_tx.send(var_value.clone()).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Read the variable: send name on "read" port
  let _ = read_tx
    .send(Arc::new(var_name) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
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
async fn test_variable_node_read_nonexistent() {
  let node = VariableNode::new("test_variable".to_string());

  let (_config_tx, read_tx, _write_tx, _value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Try to read a variable that doesn't exist
  let var_name = "nonexistent".to_string();
  let _ = read_tx
    .send(Arc::new(var_name.clone()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect error
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
  assert!(errors[0].contains("Variable 'nonexistent' not found"));
}

#[tokio::test]
async fn test_variable_node_write_string() {
  let node = VariableNode::new("test_variable".to_string());

  let (_config_tx, read_tx, write_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Write a string variable
  let var_name = "message".to_string();
  let var_value: Arc<dyn Any + Send + Sync> = Arc::new("Hello, World!".to_string());

  let _ = write_tx
    .send(Arc::new(var_name.clone()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  let _ = value_tx.send(var_value).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Read the variable
  let _ = read_tx
    .send(Arc::new(var_name) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
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
  assert_eq!(&*results[0], "Hello, World!");
}

#[tokio::test]
async fn test_variable_node_overwrite() {
  let node = VariableNode::new("test_variable".to_string());

  let (_config_tx, read_tx, write_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  let var_name = "counter".to_string();

  // Write initial value
  let _ = write_tx
    .send(Arc::new(var_name.clone()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  let _ = value_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Overwrite with new value
  let _ = write_tx
    .send(Arc::new(var_name.clone()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  let _ = value_tx
    .send(Arc::new(20i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Read the variable (should get the new value)
  let _ = read_tx
    .send(Arc::new(var_name) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
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
  assert_eq!(results[0], 20);
}

#[tokio::test]
async fn test_variable_node_error_invalid_read_type() {
  let node = VariableNode::new("test_variable".to_string());

  let (_config_tx, read_tx, _write_tx, _value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid type (not String) on read port
  let _ = read_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect error
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
  assert!(errors[0].contains("Read port expects String"));
}

#[tokio::test]
async fn test_variable_node_error_value_without_write() {
  let node = VariableNode::new("test_variable".to_string());

  let (_config_tx, _read_tx, _write_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send value without a pending write
  let _ = value_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect error
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
  assert!(errors[0].contains("no pending write operation"));
}
