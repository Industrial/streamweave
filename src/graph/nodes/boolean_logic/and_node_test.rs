//! # And Node Test Suite
//!
//! Comprehensive test suite for the AndNode implementation.

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::boolean_logic::and_node::AndNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

type ConfigSender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels
fn create_dual_input_streams() -> (ConfigSender, ConfigSender, InputStreams) {
  let (in1_tx, in1_rx) = mpsc::channel(10);
  let (in2_tx, in2_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "in1".to_string(),
    Box::pin(ReceiverStream::new(in1_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in2".to_string(),
    Box::pin(ReceiverStream::new(in2_rx)) as crate::graph::node::InputStream,
  );

  (in1_tx, in2_tx, inputs)
}

#[tokio::test]
async fn test_and_node_creation() {
  let node = AndNode::new("test_and".to_string());
  assert_eq!(node.name(), "test_and");
  assert!(node.has_input_port("in1"));
  assert!(node.has_input_port("in2"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_and_node_basic_logic() {
  let node = AndNode::new("test_and".to_string());

  let (in1_tx, in2_tx, inputs) = create_dual_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send boolean values
  let _ = in1_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;

  // Give the node time to process
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Collect results - wait for output with a longer timeout
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  // Wait for the result with a reasonable timeout
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        match result {
          Some(item) => {
            if let Ok(arc_bool) = item.downcast::<bool>() {
              results.push(*arc_bool);
              // Continue to collect all results, but break if we got at least one
              if !results.is_empty() {
                break;
              }
            }
          }
          None => break, // Stream ended
        }
      }
      _ = &mut timeout => {
        break; // Timeout
      }
    }
  }

  assert_eq!(results.len(), 1);
  assert!(results[0]); // true && true = true
}

#[tokio::test]
async fn test_and_node_false_result() {
  let node = AndNode::new("test_and".to_string());

  let (in1_tx, in2_tx, inputs) = create_dual_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send boolean values: true && false = false
  let _ = in1_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(false) as Arc<dyn Any + Send + Sync>)
    .await;

  // Give the node time to process
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Collect results - wait for output with a longer timeout
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  // Wait for the result with a reasonable timeout
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        match result {
          Some(item) => {
            if let Ok(arc_bool) = item.downcast::<bool>() {
              results.push(*arc_bool);
              // Continue to collect all results, but break if we got at least one
              if !results.is_empty() {
                break;
              }
            }
          }
          None => break, // Stream ended
        }
      }
      _ = &mut timeout => {
        break; // Timeout
      }
    }
  }

  assert_eq!(results.len(), 1);
  assert!(!results[0]);
}

#[tokio::test]
async fn test_and_node_with_integers() {
  let node = AndNode::new("test_and".to_string());

  let (in1_tx, in2_tx, inputs) = create_dual_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send integer values (non-zero = true, zero = false)
  let _ = in1_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Give the node time to process
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Collect results - wait for output with a longer timeout
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  // Wait for the result with a reasonable timeout
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        match result {
          Some(item) => {
            if let Ok(arc_bool) = item.downcast::<bool>() {
              results.push(*arc_bool);
              // Continue to collect all results, but break if we got at least one
              if !results.is_empty() {
                break;
              }
            }
          }
          None => break, // Stream ended
        }
      }
      _ = &mut timeout => {
        break; // Timeout
      }
    }
  }

  assert_eq!(results.len(), 1);
  assert!(!results[0]); // 5 (true) && 0 (false) = false
}
