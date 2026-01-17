//! Tests for ReadVariableNode

use crate::node::{InputStreams, Node};
use crate::nodes::read_variable_node::ReadVariableNode;
use crate::nodes::variable_node::VariableNode;
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
  let (name_tx, name_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "name".to_string(),
    Box::pin(ReceiverStream::new(name_rx)) as crate::node::InputStream,
  );

  (config_tx, name_tx, inputs)
}

#[tokio::test]
async fn test_read_variable_node_creation() {
  let node = ReadVariableNode::new("test_read_var".to_string());
  assert_eq!(node.name(), "test_read_var");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("name"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
  assert!(!node.has_config());
}

#[tokio::test]
async fn test_read_variable_node_read_existing() {
  // Create VariableNode and write a variable
  let var_node = VariableNode::new("vars".to_string());
  let var_store = var_node.variables();

  // Manually write a variable to the store
  {
    let mut vars = var_store.lock().await;
    vars.insert(
      "counter".to_string(),
      Arc::new(42i32) as Arc<dyn Any + Send + Sync>,
    );
  }

  // Create ReadVariableNode
  let read_node = ReadVariableNode::new("read_var".to_string());
  let (config_tx, name_tx, inputs) = create_input_streams();
  let outputs_future = read_node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration (variable store)
  let _ = config_tx
    .send(Arc::new(var_store) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send variable name to read
  let _ = name_tx
    .send(Arc::new("counter".to_string()) as Arc<dyn Any + Send + Sync>)
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
async fn test_read_variable_node_read_nonexistent() {
  // Create VariableNode
  let var_node = VariableNode::new("vars".to_string());
  let var_store = var_node.variables();

  // Create ReadVariableNode
  let read_node = ReadVariableNode::new("read_var".to_string());
  let (config_tx, name_tx, inputs) = create_input_streams();
  let outputs_future = read_node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration (variable store)
  let _ = config_tx
    .send(Arc::new(var_store) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Try to read a nonexistent variable
  let _ = name_tx
    .send(Arc::new("nonexistent".to_string()) as Arc<dyn Any + Send + Sync>)
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
async fn test_read_variable_node_error_no_config() {
  let read_node = ReadVariableNode::new("read_var".to_string());
  let (_config_tx, name_tx, inputs) = create_input_streams();
  let outputs_future = read_node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Try to read without configuration
  let _ = name_tx
    .send(Arc::new("counter".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("No variable store configured"));
}

#[tokio::test]
async fn test_read_variable_node_error_invalid_name_type() {
  // Create VariableNode
  let var_node = VariableNode::new("vars".to_string());
  let var_store = var_node.variables();

  // Create ReadVariableNode
  let read_node = ReadVariableNode::new("read_var".to_string());
  let (config_tx, name_tx, inputs) = create_input_streams();
  let outputs_future = read_node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(var_store) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send invalid type (not String) on name port
  let _ = name_tx
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
  assert!(errors[0].contains("Name port expects String"));
}

#[tokio::test]
async fn test_read_variable_node_read_string() {
  // Create VariableNode and write a string variable
  let var_node = VariableNode::new("vars".to_string());
  let var_store = var_node.variables();

  // Manually write a variable to the store
  {
    let mut vars = var_store.lock().await;
    vars.insert(
      "message".to_string(),
      Arc::new("Hello, World!".to_string()) as Arc<dyn Any + Send + Sync>,
    );
  }

  // Create ReadVariableNode
  let read_node = ReadVariableNode::new("read_var".to_string());
  let (config_tx, name_tx, inputs) = create_input_streams();
  let outputs_future = read_node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(var_store) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send variable name to read
  let _ = name_tx
    .send(Arc::new("message".to_string()) as Arc<dyn Any + Send + Sync>)
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
