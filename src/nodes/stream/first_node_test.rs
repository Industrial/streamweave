#![allow(unused_imports, dead_code, unused, clippy::type_complexity)]

use super::FirstNode;
use crate::node::{InputStreams, Node, OutputStreams};
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

  (config_tx, in_tx, inputs)
}

#[tokio::test]
async fn test_first_node_creation() {
  let node = FirstNode::new("test_first".to_string());
  assert_eq!(node.name(), "test_first");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_first_node_emits_only_first_item() {
  let node = FirstNode::new("test_first".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send multiple items: 10, 20, 30, 40, 50
  let test_values = vec![10i32, 20i32, 30i32, 40i32, 50i32];
  for value in test_values {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have received only the first item
  assert_eq!(results.len(), 1);
  if let Ok(value) = results[0].clone().downcast::<i32>() {
    assert_eq!(*value, 10i32);
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
async fn test_first_node_empty_stream() {
  let node = FirstNode::new("test_first".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send no items (empty stream)
  drop(in_tx); // Close the input stream immediately

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have received no items (empty stream)
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_first_node_single_item() {
  let node = FirstNode::new("test_first".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send single item
  let test_value = 42i32;
  in_tx
    .send(Arc::new(test_value) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  drop(in_tx); // Close the input stream

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have received the single item
  assert_eq!(results.len(), 1);
  if let Ok(value) = results[0].clone().downcast::<i32>() {
    assert_eq!(*value, test_value);
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
async fn test_first_node_string_values() {
  let node = FirstNode::new("test_first".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send string values: "first", "second", "third"
  let test_values = vec!["first", "second", "third"];
  for value in test_values {
    in_tx
      .send(Arc::new(value.to_string()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have received only the first string
  assert_eq!(results.len(), 1);
  if let Ok(value) = results[0].clone().downcast::<String>() {
    assert_eq!(*value, "first");
  } else {
    panic!("Result is not a String");
  }
}
