//! Tests for IsFloatNode

use crate::node::{InputStreams, Node};
use crate::nodes::type_ops::IsFloatNode;
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
async fn test_is_float_node_creation() {
  let node = IsFloatNode::new("test_is_float".to_string());
  assert_eq!(node.name(), "test_is_float");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_is_float_f32() {
  let node = IsFloatNode::new("test_is_float".to_string());

  let (config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send f32
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_tx
    .send(Arc::new(3.14f32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channels
  drop(config_tx);
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_bool) = result.downcast::<bool>() {
      results.push(*arc_bool);
    }
  }

  assert_eq!(results.len(), 1);
  assert!(results[0]); // f32 should be true
}

#[tokio::test]
async fn test_is_float_f64() {
  let node = IsFloatNode::new("test_is_float".to_string());

  let (config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send f64
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_tx
    .send(Arc::new(2.71828f64) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channels
  drop(config_tx);
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_bool) = result.downcast::<bool>() {
      results.push(*arc_bool);
    }
  }

  assert_eq!(results.len(), 1);
  assert!(results[0]); // f64 should be true
}

#[tokio::test]
async fn test_is_float_i32() {
  let node = IsFloatNode::new("test_is_float".to_string());

  let (config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send i32
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channels
  drop(config_tx);
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_bool) = result.downcast::<bool>() {
      results.push(*arc_bool);
    }
  }

  assert_eq!(results.len(), 1);
  assert!(!results[0]); // i32 should be false
}

#[tokio::test]
async fn test_is_float_string() {
  let node = IsFloatNode::new("test_is_float".to_string());

  let (config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send string
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channels
  drop(config_tx);
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_bool) = result.downcast::<bool>() {
      results.push(*arc_bool);
    }
  }

  assert_eq!(results.len(), 1);
  assert!(!results[0]); // string should be false
}

#[tokio::test]
async fn test_is_float_bool() {
  let node = IsFloatNode::new("test_is_float".to_string());

  let (config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send boolean
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channels
  drop(config_tx);
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_bool) = result.downcast::<bool>() {
      results.push(*arc_bool);
    }
  }

  assert_eq!(results.len(), 1);
  assert!(!results[0]); // boolean should be false
}

#[tokio::test]
async fn test_is_float_multiple_types() {
  let node = IsFloatNode::new("test_is_float".to_string());

  let (config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send multiple types
  let _ = input_tx
    .send(Arc::new(3.14f32) as Arc<dyn Any + Send + Sync>)
    .await; // f32 -> true
  let _ = input_tx
    .send(Arc::new(2.718f64) as Arc<dyn Any + Send + Sync>)
    .await; // f64 -> true
  let _ = input_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await; // i32 -> false
  let _ = input_tx
    .send(Arc::new("test".to_string()) as Arc<dyn Any + Send + Sync>)
    .await; // string -> false

  // Close input channels
  drop(config_tx);
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_bool) = result.downcast::<bool>() {
      results.push(*arc_bool);
    }
  }

  assert_eq!(results.len(), 4);
  assert!(results[0]); // f32 -> true
  assert!(results[1]); // f64 -> true
  assert!(!results[2]); // i32 -> false
  assert!(!results[3]); // string -> false
}
