//! Tests for StringAppendNode

use crate::node::{InputStreams, Node};
use crate::nodes::string::StringAppendNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

type AnySender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels
fn create_input_streams() -> (AnySender, AnySender, AnySender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (base_tx, base_rx) = mpsc::channel(10);
  let (suffix_tx, suffix_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(base_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "suffix".to_string(),
    Box::pin(ReceiverStream::new(suffix_rx)) as crate::node::InputStream,
  );

  (config_tx, base_tx, suffix_tx, inputs)
}

#[tokio::test]
async fn test_string_append_node_creation() {
  let node = StringAppendNode::new("test_append".to_string());
  assert_eq!(node.name(), "test_append");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("suffix"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_string_append_basic() {
  let node = StringAppendNode::new("test_append".to_string());

  let (_config_tx, base_tx, suffix_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello" + " World" = "Hello World"
  let _ = base_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = suffix_tx
    .send(Arc::new(" World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result
        && let Ok(arc_str) = item.downcast::<String>() {
          results.push((*arc_str).clone());
        }
    }
    _ = &mut timeout => {},
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], "Hello World");
}

#[tokio::test]
async fn test_string_append_empty_base() {
  let node = StringAppendNode::new("test_append".to_string());

  let (_config_tx, base_tx, suffix_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "" + "suffix" = "suffix"
  let _ = base_tx
    .send(Arc::new("".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = suffix_tx
    .send(Arc::new("suffix".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result
        && let Ok(arc_str) = item.downcast::<String>() {
          results.push((*arc_str).clone());
        }
    }
    _ = &mut timeout => {},
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], "suffix");
}

#[tokio::test]
async fn test_string_append_unicode() {
  let node = StringAppendNode::new("test_append".to_string());

  let (_config_tx, base_tx, suffix_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello " + "🌍" = "Hello 🌍"
  let _ = base_tx
    .send(Arc::new("Hello ".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = suffix_tx
    .send(Arc::new("🌍".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  tokio::select! {
    result = stream.next() => {
      if let Some(item) = result
        && let Ok(arc_str) = item.downcast::<String>() {
          results.push((*arc_str).clone());
        }
    }
    _ = &mut timeout => {},
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], "Hello 🌍");
}
