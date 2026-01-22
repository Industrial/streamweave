//! Tests for ObjectSizeNode

use crate::node::{InputStreams, Node};
use crate::nodes::object::object_size_node::ObjectSizeNode;
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
  let (input_tx, input_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(input_rx)) as crate::node::InputStream,
  );

  (config_tx, input_tx, inputs)
}

#[tokio::test]
async fn test_object_size_node_creation() {
  let node = ObjectSizeNode::new("test_object_size".to_string());
  assert_eq!(node.name(), "test_object_size");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_object_size_node_basic() {
  let node = ObjectSizeNode::new("test_object_size".to_string());

  let (_config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a test object with 3 properties
  let mut test_obj = HashMap::new();
  test_obj.insert(
    "name".to_string(),
    Arc::new("Alice".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  test_obj.insert(
    "age".to_string(),
    Arc::new(30i64) as Arc<dyn Any + Send + Sync>,
  );
  test_obj.insert(
    "active".to_string(),
    Arc::new(true) as Arc<dyn Any + Send + Sync>,
  );

  // Send the object
  let _ = input_tx
    .send(Arc::new(test_obj) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channel
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_size) = result.downcast::<i64>() {
      results.push(*arc_size);
    }
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], 3); // Object has 3 properties
}

#[tokio::test]
async fn test_object_size_node_multiple_objects() {
  let node = ObjectSizeNode::new("test_object_size".to_string());

  let (_config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create test objects with different sizes
  let mut obj1 = HashMap::new();
  obj1.insert(
    "a".to_string(),
    Arc::new(1i64) as Arc<dyn Any + Send + Sync>,
  );

  let mut obj2 = HashMap::new();
  obj2.insert(
    "x".to_string(),
    Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  obj2.insert(
    "y".to_string(),
    Arc::new("world".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  obj2.insert(
    "z".to_string(),
    Arc::new(42i64) as Arc<dyn Any + Send + Sync>,
  );

  let obj3: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new(); // Empty object

  // Send the objects
  let _ = input_tx
    .send(Arc::new(obj1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_tx
    .send(Arc::new(obj2) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_tx
    .send(Arc::new(obj3) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channel
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_size) = result.downcast::<i64>() {
      results.push(*arc_size);
    }
  }

  assert_eq!(results.len(), 3);
  assert_eq!(results[0], 1); // obj1 has 1 property
  assert_eq!(results[1], 3); // obj2 has 3 properties
  assert_eq!(results[2], 0); // obj3 has 0 properties (empty)
}

#[tokio::test]
async fn test_object_size_node_error_handling() {
  let node = ObjectSizeNode::new("test_object_size".to_string());

  let (_config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send a non-object value (should cause error)
  let _ = input_tx
    .send(Arc::new("not an object".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channel
  drop(input_tx);

  // Collect results from both streams
  let out_stream = outputs.remove("out").unwrap();
  let error_stream = outputs.remove("error").unwrap();

  let mut out_results = Vec::new();
  let mut error_results = Vec::new();

  // Collect from out stream
  let mut out_stream = out_stream;
  while let Some(result) = out_stream.next().await {
    if let Ok(arc_size) = result.downcast::<i64>() {
      out_results.push(*arc_size);
    }
  }

  // Collect from error stream
  let mut error_stream = error_stream;
  while let Some(result) = error_stream.next().await {
    if let Ok(error_msg) = result.downcast::<String>() {
      error_results.push((*error_msg).clone());
    }
  }

  assert_eq!(out_results.len(), 0); // No successful outputs
  assert_eq!(error_results.len(), 1); // One error
  assert!(error_results[0].contains("Unsupported type for object size input"));
}
