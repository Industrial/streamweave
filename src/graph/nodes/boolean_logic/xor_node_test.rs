//! # Xor Node Test Suite

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::boolean_logic::xor_node::XorNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

type ConfigSender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

fn create_dual_input_streams() -> (ConfigSender, ConfigSender, InputStreams) {
  let (_config_tx, config_rx) = mpsc::channel(10);
  let (in1_tx, in1_rx) = mpsc::channel(10);
  let (in2_tx, in2_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
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
async fn test_xor_node_creation() {
  let node = XorNode::new("test_xor".to_string());
  assert_eq!(node.name(), "test_xor");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in1"));
  assert!(node.has_input_port("in2"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_xor_node_different_inputs() {
  let node = XorNode::new("test_xor".to_string());
  let (in1_tx, in2_tx, inputs) = create_dual_input_streams();

  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // true XOR false = true
  let _ = in1_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(false) as Arc<dyn Any + Send + Sync>)
    .await;

  // Give the node time to process
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        match result {
          Some(item) => {
            if let Ok(arc_bool) = item.downcast::<bool>() {
              results.push(*arc_bool);
              break;
            }
          }
          None => break,
        }
      }
      _ = &mut timeout => {
        break;
      }
    }
  }

  assert_eq!(results.len(), 1);
  assert!(results[0]);
}

#[tokio::test]
async fn test_xor_node_same_inputs() {
  let node = XorNode::new("test_xor".to_string());
  let (in1_tx, in2_tx, inputs) = create_dual_input_streams();

  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // true XOR true = false
  let _ = in1_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;

  // Give the node time to process
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        match result {
          Some(item) => {
            if let Ok(arc_bool) = item.downcast::<bool>() {
              results.push(*arc_bool);
              break;
            }
          }
          None => break,
        }
      }
      _ = &mut timeout => {
        break;
      }
    }
  }

  assert_eq!(results.len(), 1);
  assert!(!results[0]);
}
