//! Tests for StringFormatNode

use crate::node::{InputStreams, Node};
use crate::nodes::string::StringFormatNode;
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
  let (template_tx, template_rx) = mpsc::channel(10);
  let (value_tx, value_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "template".to_string(),
    Box::pin(ReceiverStream::new(template_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "value".to_string(),
    Box::pin(ReceiverStream::new(value_rx)) as crate::node::InputStream,
  );

  (config_tx, template_tx, value_tx, inputs)
}

#[tokio::test]
async fn test_string_format_node_creation() {
  let node = StringFormatNode::new("test_format".to_string());
  assert_eq!(node.name(), "test_format");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("template"));
  assert!(node.has_input_port("value"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_string_format_basic() {
  let node = StringFormatNode::new("test_format".to_string());

  let (_config_tx, template_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send template and value: "Hello {}" + "World" = "Hello World"
  let _ = template_tx
    .send(Arc::new("Hello {}".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new("World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push((*arc_str).clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], "Hello World");
}

#[tokio::test]
async fn test_string_format_with_number() {
  let node = StringFormatNode::new("test_format".to_string());

  let (_config_tx, template_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send template and numeric value: "Count: {}" + 42 = "Count: 42"
  let _ = template_tx
    .send(Arc::new("Count: {}".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push((*arc_str).clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], "Count: 42");
}

#[tokio::test]
async fn test_string_format_multiple_placeholders() {
  let node = StringFormatNode::new("test_format".to_string());

  let (_config_tx, template_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send template with multiple placeholders and single value
  // "User: {} logged in at {}" + "Alice" = "User: Alice logged in at Alice"
  let _ = template_tx
    .send(Arc::new("User: {} logged in at {}".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new("Alice".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push((*arc_str).clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], "User: Alice logged in at Alice");
}

#[tokio::test]
async fn test_string_format_no_placeholder() {
  let node = StringFormatNode::new("test_format".to_string());

  let (_config_tx, template_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send template without placeholder: "Hello World" + "ignored" = "Hello World"
  let _ = template_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new("ignored".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push((*arc_str).clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  assert_eq!(results[0], "Hello World");
}
