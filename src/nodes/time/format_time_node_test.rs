//! Tests for FormatTimeNode

use crate::node::{InputStreams, Node};
use crate::nodes::time::FormatTimeNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
type AnySender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels
fn create_input_streams() -> (AnySender, AnySender, AnySender, InputStreams) {
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
async fn test_format_time_node_creation() {
  let node = FormatTimeNode::new("test_format_time".to_string());
  assert_eq!(node.name(), "test_format_time");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("format"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_format_time_node_default_format() {
  let node = FormatTimeNode::new("test_format_time".to_string());

  let (_config_tx, input_tx, _format_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Test timestamp for "2024-01-15T09:50:45.123Z" (RFC3339)
  // This is 1705312245123 milliseconds since Unix epoch
  let test_timestamp = 1705312245123i64;

  // Send the timestamp
  let _ = input_tx
    .send(Arc::new(test_timestamp) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channel
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_formatted) = result.downcast::<String>() {
      results.push((*arc_formatted).clone());
    }
  }

  assert_eq!(results.len(), 1);
  // Should be in RFC3339 format (default)
  assert_eq!(results[0], "2024-01-15T09:50:45.123Z");
}

#[tokio::test]
async fn test_format_time_node_custom_format() {
  let node = FormatTimeNode::new("test_format_time".to_string());

  let (_config_tx, input_tx, format_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Test timestamp for "2024-01-15T09:50:45.123Z"
  let test_timestamp = 1705312245123i64;
  let custom_format = "%Y-%m-%d %H:%M:%S".to_string();

  // Send format first, then timestamp
  let _ = format_tx
    .send(Arc::new(custom_format) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = input_tx
    .send(Arc::new(test_timestamp) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close input channels
  drop(input_tx);
  drop(format_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_formatted) = result.downcast::<String>() {
      results.push((*arc_formatted).clone());
    }
  }

  assert_eq!(results.len(), 1);
  // Should be in custom format
  assert_eq!(results[0], "2024-01-15 09:50:45");
}

#[tokio::test]
async fn test_format_time_node_multiple_timestamps() {
  let node = FormatTimeNode::new("test_format_time".to_string());

  let (_config_tx, input_tx, _format_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Test multiple timestamps
  let timestamps = vec![
    1705312245123i64, // 2024-01-15T09:50:45.123Z
    1609459200000i64, // 2021-01-01T00:00:00.000Z
    946684800000i64,  // 2000-01-01T00:00:00.000Z
  ];

  let expected = vec![
    "2024-01-15T09:50:45.123Z",
    "2021-01-01T00:00:00.000Z",
    "2000-01-01T00:00:00.000Z",
  ];

  // Send the timestamps
  for timestamp in timestamps {
    let _ = input_tx
      .send(Arc::new(timestamp) as Arc<dyn Any + Send + Sync>)
      .await;
  }

  // Close input channel
  drop(input_tx);

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  while let Some(result) = stream.next().await {
    if let Ok(arc_formatted) = result.downcast::<String>() {
      results.push((*arc_formatted).clone());
    }
  }

  assert_eq!(results.len(), 3);
  assert_eq!(results, expected);
}

#[tokio::test]
async fn test_format_time_node_error_handling() {
  let node = FormatTimeNode::new("test_format_time".to_string());

  let (_config_tx, input_tx, _format_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid inputs
  let invalid_inputs = vec![
    Arc::new("not a timestamp".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new(true) as Arc<dyn Any + Send + Sync>,
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
    if let Ok(arc_formatted) = result.downcast::<String>() {
      out_results.push((*arc_formatted).clone());
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
  assert_eq!(error_results.len(), 2); // Two errors
  for error in &error_results {
    assert!(error.contains("Unsupported type for timestamp input"));
  }
}

#[tokio::test]
async fn test_format_time_node_invalid_timestamp() {
  let node = FormatTimeNode::new("test_format_time".to_string());

  let (_config_tx, input_tx, _format_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send an invalid timestamp (before Unix epoch)
  let invalid_timestamp = -1i64;
  let _ = input_tx
    .send(Arc::new(invalid_timestamp) as Arc<dyn Any + Send + Sync>)
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
    if let Ok(arc_formatted) = result.downcast::<String>() {
      out_results.push((*arc_formatted).clone());
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
  assert!(error_results[0].contains("Invalid timestamp"));
}
