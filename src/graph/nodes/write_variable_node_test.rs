//! Tests for WriteVariableNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::variable_node::VariableNode;
use crate::graph::nodes::write_variable_node::WriteVariableNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
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
  let (name_tx, name_rx) = mpsc::channel(10);
  let (value_tx, value_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "name".to_string(),
    Box::pin(ReceiverStream::new(name_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "value".to_string(),
    Box::pin(ReceiverStream::new(value_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, name_tx, value_tx, inputs)
}

#[tokio::test]
async fn test_write_variable_node_creation() {
  let node = WriteVariableNode::new("test_write_var".to_string());
  assert_eq!(node.name(), "test_write_var");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("name"));
  assert!(node.has_input_port("value"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
  assert!(!node.has_config());
}

#[tokio::test]
async fn test_write_variable_node_write_and_read() {
  // Create VariableNode to get the shared store
  let var_node = VariableNode::new("vars".to_string());
  let var_store = var_node.variables();

  // Create WriteVariableNode
  let write_node = WriteVariableNode::new("write_var".to_string());
  let (config_tx, name_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = write_node.execute(inputs);
  let _outputs = outputs_future.await.unwrap();

  // Send configuration (variable store)
  let _ = config_tx
    .send(Arc::new(var_store.clone()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Write a variable: send name, then value
  let var_name = "counter".to_string();
  let var_value = Arc::new(42i32) as Arc<dyn Any + Send + Sync>;

  let _ = name_tx
    .send(Arc::new(var_name.clone()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  let _ = value_tx.send(var_value.clone()).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Verify the variable was written by reading it directly from the store
  {
    let vars = var_store.lock().await;
    if let Some(value) = vars.get(&var_name) {
      if let Ok(arc_i32) = value.clone().downcast::<i32>() {
        assert_eq!(*arc_i32, 42);
      } else {
        panic!("Value type mismatch");
      }
    } else {
      panic!("Variable not found in store");
    }
  }

  // The write was successful - we verified it by reading directly from the store above
}

#[tokio::test]
async fn test_write_variable_node_write_string() {
  // Create VariableNode to get the shared store
  let var_node = VariableNode::new("vars".to_string());
  let var_store = var_node.variables();

  // Create WriteVariableNode
  let write_node = WriteVariableNode::new("write_var".to_string());
  let (config_tx, name_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = write_node.execute(inputs);
  let _outputs = outputs_future.await.unwrap(); // Keep outputs alive

  // Send configuration
  let _ = config_tx
    .send(Arc::new(var_store.clone()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Write a string variable
  let var_name = "message".to_string();
  let var_value: Arc<dyn Any + Send + Sync> = Arc::new("Hello, World!".to_string());

  let _ = name_tx
    .send(Arc::new(var_name.clone()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  let _ = value_tx.send(var_value).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Verify the variable was written
  {
    let vars = var_store.lock().await;
    if let Some(value) = vars.get(&var_name) {
      if let Ok(arc_string) = value.clone().downcast::<String>() {
        assert_eq!(&*arc_string, "Hello, World!");
      } else {
        panic!("Value type mismatch");
      }
    } else {
      panic!("Variable not found in store");
    }
  }
}

#[tokio::test]
async fn test_write_variable_node_overwrite() {
  // Create VariableNode to get the shared store
  let var_node = VariableNode::new("vars".to_string());
  let var_store = var_node.variables();

  // Create WriteVariableNode
  let write_node = WriteVariableNode::new("write_var".to_string());
  let (config_tx, name_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = write_node.execute(inputs);
  let _outputs = outputs_future.await.unwrap(); // Keep outputs alive

  // Send configuration
  let _ = config_tx
    .send(Arc::new(var_store.clone()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  let var_name = "counter".to_string();

  // Write initial value
  let _ = name_tx
    .send(Arc::new(var_name.clone()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  let _ = value_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  // Overwrite with new value
  let _ = name_tx
    .send(Arc::new(var_name.clone()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  let _ = value_tx
    .send(Arc::new(20i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  // Verify the variable was overwritten
  {
    let vars = var_store.lock().await;
    if let Some(value) = vars.get(&var_name) {
      if let Ok(arc_i32) = value.clone().downcast::<i32>() {
        assert_eq!(*arc_i32, 20);
      } else {
        panic!("Value type mismatch");
      }
    } else {
      panic!("Variable not found in store");
    }
  }
}

#[tokio::test]
async fn test_write_variable_node_error_no_config() {
  let write_node = WriteVariableNode::new("write_var".to_string());
  let (_config_tx, name_tx, _value_tx, inputs) = create_input_streams();
  let outputs_future = write_node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Try to write without configuration
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
async fn test_write_variable_node_error_invalid_name_type() {
  // Create VariableNode to get the shared store
  let var_node = VariableNode::new("vars".to_string());
  let var_store = var_node.variables();

  // Create WriteVariableNode
  let write_node = WriteVariableNode::new("write_var".to_string());
  let (config_tx, name_tx, _value_tx, inputs) = create_input_streams();
  let outputs_future = write_node.execute(inputs);
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
async fn test_write_variable_node_error_value_without_name() {
  // Create VariableNode to get the shared store
  let var_node = VariableNode::new("vars".to_string());
  let var_store = var_node.variables();

  // Create WriteVariableNode
  let write_node = WriteVariableNode::new("write_var".to_string());
  let (config_tx, _name_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = write_node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(var_store) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send value without a pending name
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
