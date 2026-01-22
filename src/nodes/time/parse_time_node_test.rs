//! Tests for ParseTimeNode

use crate::node::{InputStreams, Node};
use crate::nodes::time::ParseTimeNode;
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
  let (input_tx, input_rx) = mpsc::channel(10);
  let (format_tx, format_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(input_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "format".to_string(),
    Box::pin(ReceiverStream::new(format_rx)) as crate::node::InputStream,
  );

  (config_tx, input_tx, format_tx, inputs)
}

#[tokio::test]
async fn test_parse_time_node_creation() {
  let node = ParseTimeNode::new("test_parse_time".to_string());
  assert_eq!(node.name(), "test_parse_time");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("format"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_parse_time_node_rfc3339() {
  let node = ParseTimeNode::new("test_parse_time".to_string());

  let (_config_tx, input_tx, _format_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Test RFC3339 timestamp
  let time_str = "2024-01-15T09:50:45.123Z".to_string();
  let expected_timestamp = 1705312245123i64; // This should be the parsed timestamp

  // Send the time string
  let _ = input_tx
    .send(Arc::new(time_str) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channel
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_timestamp) = result.downcast::<i64>() {
      results.push(*arc_timestamp);
    }
  }

  assert_eq!(results.len(), 1);
  // Should parse to the expected timestamp (allowing some tolerance for different parsing)
  assert!((results[0] - expected_timestamp).abs() < 1000);
}

#[tokio::test]
async fn test_parse_time_node_custom_format() {
  let node = ParseTimeNode::new("test_parse_time".to_string());

  let (_config_tx, input_tx, format_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Test custom format
  let time_str = "2024-01-15 09:50:45".to_string();
  let custom_format = "%Y-%m-%d %H:%M:%S".to_string();

  // Send format first, then time string
  let _ = format_tx
    .send(Arc::new(custom_format) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_tx
    .send(Arc::new(time_str) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channels
  drop(input_tx);
  drop(format_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_timestamp) = result.downcast::<i64>() {
      results.push(*arc_timestamp);
    }
  }

  assert_eq!(results.len(), 1);
  // Should parse to approximately 2024-01-15T09:50:45 UTC
  let expected_timestamp = 1705312245000i64; // Approximate timestamp
  assert!((results[0] - expected_timestamp).abs() < 1000);
}

#[tokio::test]
async fn test_parse_time_node_multiple_formats() {
  let node = ParseTimeNode::new("test_parse_time".to_string());

  let (_config_tx, input_tx, format_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Test date-only format
  let time_str = "2024-01-15".to_string();
  let format_str = "%Y-%m-%d".to_string();

  // Send format first
  let _ = format_tx
    .send(Arc::new(format_str) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send time string
  let _ = input_tx
    .send(Arc::new(time_str) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channels
  drop(input_tx);
  drop(format_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_timestamp) = result.downcast::<i64>() {
      results.push(*arc_timestamp);
    }
  }

  assert_eq!(results.len(), 1);
  // Should parse successfully
  assert!(results[0] > 1700000000000i64); // Should be in 2024
}

#[tokio::test]
async fn test_parse_time_node_error_handling() {
  let node = ParseTimeNode::new("test_parse_time".to_string());

  let (_config_tx, input_tx, _format_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid inputs
  let invalid_inputs = vec![
    Arc::new("not a timestamp".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("invalid-date-format".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new(12345i64) as Arc<dyn Any + Send + Sync>, // Wrong type
  ];

  for invalid_input in invalid_inputs {
    let _ = input_tx.send(invalid_input).await;
  }

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
    if let Ok(arc_timestamp) = result.downcast::<i64>() {
      out_results.push(*arc_timestamp);
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
  assert_eq!(error_results.len(), 3); // Three errors
  for error in &error_results {
    assert!(error.contains("Failed to parse") || error.contains("Unsupported type"));
  }
}

#[tokio::test]
async fn test_parse_time_node_invalid_format() {
  let node = ParseTimeNode::new("test_parse_time".to_string());

  let (_config_tx, input_tx, format_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid format
  let invalid_format = "%invalid-format".to_string();
  let time_str = "2024-01-15T09:50:45Z".to_string();

  let _ = format_tx
    .send(Arc::new(invalid_format) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_tx
    .send(Arc::new(time_str) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channels
  drop(input_tx);
  drop(format_tx);

  // Collect results from both streams
  let out_stream = outputs.remove("out").unwrap();
  let error_stream = outputs.remove("error").unwrap();

  let mut out_results = Vec::new();
  let mut error_results = Vec::new();

  // Collect from out stream
  let mut out_stream = out_stream;
  while let Some(result) = out_stream.next().await {
    if let Ok(arc_timestamp) = result.downcast::<i64>() {
      out_results.push(*arc_timestamp);
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
  assert!(error_results[0].contains("Failed to parse"));
}
