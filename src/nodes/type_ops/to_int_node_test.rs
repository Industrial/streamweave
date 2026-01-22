//! Tests for ToIntNode

use crate::node::{InputStreams, Node};
use crate::nodes::type_ops::ToIntNode;
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
async fn test_to_int_node_creation() {
  let node = ToIntNode::new("test_to_int".to_string());
  assert_eq!(node.name(), "test_to_int");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_to_int_node_integer() {
  let node = ToIntNode::new("test_to_int".to_string());

  let (_config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send integer
  let _ = input_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channel
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_int) = result.downcast::<i64>() {
      results.push(*arc_int);
    }
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], 42i64);
}

#[tokio::test]
async fn test_to_int_node_float() {
  let node = ToIntNode::new("test_to_int".to_string());

  let (_config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send float
  let _ = input_tx
    .send(Arc::new(3.14f64) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channel
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_int) = result.downcast::<i64>() {
      results.push(*arc_int);
    }
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], 3i64); // 3.14 truncated to 3
}

#[tokio::test]
async fn test_to_int_node_boolean() {
  let node = ToIntNode::new("test_to_int".to_string());

  let (_config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send booleans
  let _ = input_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_tx
    .send(Arc::new(false) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channel
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_int) = result.downcast::<i64>() {
      results.push(*arc_int);
    }
  }

  assert_eq!(results.len(), 2);
  assert_eq!(results[0], 1i64); // true -> 1
  assert_eq!(results[1], 0i64); // false -> 0
}

#[tokio::test]
async fn test_to_int_node_string() {
  let node = ToIntNode::new("test_to_int".to_string());

  let (_config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send valid string
  let _ = input_tx
    .send(Arc::new("42".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send invalid string
  let _ = input_tx
    .send(Arc::new("not_a_number".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channel
  drop(input_tx);

  // Collect results from both streams
  let out_stream = outputs.remove("out").unwrap();
  let error_stream = outputs.remove("error").unwrap();

  let mut out_results = Vec::new();
  let mut error_count = 0;

  // Collect from out stream
  let mut out_stream = out_stream;
  while let Some(result) = out_stream.next().await {
    if let Ok(arc_int) = result.downcast::<i64>() {
      out_results.push(*arc_int);
    }
  }

  // Collect from error stream
  let mut error_stream = error_stream;
  while let Some(_result) = error_stream.next().await {
    error_count += 1;
  }

  assert_eq!(out_results.len(), 1); // Only valid string parsed
  assert_eq!(error_count, 1); // Invalid string caused error
  assert_eq!(out_results[0], 42i64);
}

#[tokio::test]
async fn test_to_int_node_error_handling() {
  let node = ToIntNode::new("test_to_int".to_string());

  let (_config_tx, input_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send unsupported type
  let _ = input_tx
    .send(Arc::new(vec![1, 2, 3]) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channel
  drop(input_tx);

  // Collect results from both streams
  let out_stream = outputs.remove("out").unwrap();
  let error_stream = outputs.remove("error").unwrap();

  let mut out_results = Vec::new();
  let mut error_count = 0;

  // Collect from out stream
  let mut out_stream = out_stream;
  while let Some(_result) = out_stream.next().await {
    out_results.push(0i64);
  }

  // Collect from error stream
  let mut error_stream = error_stream;
  while let Some(_result) = error_stream.next().await {
    error_count += 1;
  }

  assert_eq!(out_results.len(), 0); // No successful outputs
  assert_eq!(error_count, 1); // One error for unsupported type
}