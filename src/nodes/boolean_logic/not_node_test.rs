//! # Not Node Test Suite

use crate::node::{InputStreams, Node};
use crate::nodes::boolean_logic::not_node::NotNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

fn create_single_input_stream() -> (mpsc::Sender<Arc<dyn Any + Send + Sync>>, InputStreams) {
  let (_config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::node::InputStream,
  );

  (in_tx, inputs)
}

#[tokio::test]
async fn test_not_node_creation() {
  let node = NotNode::new("test_not".to_string());
  assert_eq!(node.name(), "test_not");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_not_node_inverts_true() {
  let node = NotNode::new("test_not".to_string());
  let (in_tx, inputs) = create_single_input_stream();

  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  let _ = in_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result
        && let Ok(arc_bool) = item.downcast::<bool>() {
          results.push(*arc_bool);
        }
    }
    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
  }

  assert_eq!(results.len(), 1);
  assert!(!results[0]); // !true = false
}

#[tokio::test]
async fn test_not_node_inverts_false() {
  let node = NotNode::new("test_not".to_string());
  let (in_tx, inputs) = create_single_input_stream();

  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  let _ = in_tx
    .send(Arc::new(false) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result
        && let Ok(arc_bool) = item.downcast::<bool>() {
          results.push(*arc_bool);
        }
    }
    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
  }

  assert_eq!(results.len(), 1);
  assert!(results[0]); // !false = true
}
