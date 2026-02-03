//! Tests for StringCharAtNode

use crate::node::{InputStreams, Node};
use crate::nodes::string::StringCharAtNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

type AnySender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels
fn create_input_streams() -> (AnySender, AnySender, AnySender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (index_tx, index_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "index".to_string(),
    Box::pin(ReceiverStream::new(index_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, index_tx, inputs)
}

#[tokio::test]
async fn test_string_char_at_node_creation() {
  let node = StringCharAtNode::new("test_char_at".to_string());
  assert_eq!(node.name(), "test_char_at");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("index"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_string_char_at_basic() {
  let node = StringCharAtNode::new("test_char_at".to_string());

  let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello"[1] -> "e"
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = index_tx
    .send(Arc::new(1usize) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0], "e");
}

#[tokio::test]
async fn test_string_char_at_first_character() {
  let node = StringCharAtNode::new("test_char_at".to_string());

  let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello"[0] -> "H"
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = index_tx
    .send(Arc::new(0usize) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0], "H");
}

#[tokio::test]
async fn test_string_char_at_unicode() {
  let node = StringCharAtNode::new("test_char_at".to_string());

  let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "🚀🚁"[1] -> "🚁"
  let _ = in_tx
    .send(Arc::new("🚀🚁".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = index_tx
    .send(Arc::new(1usize) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0], "🚁");
}

#[tokio::test]
async fn test_string_char_at_different_index_types() {
  // Test different index types that can be converted to usize
  let test_cases = vec![
    ("u32", 1u32 as usize, "e"),
    ("u64", 1u64 as usize, "e"),
    ("i32", 1i32 as usize, "e"),
    ("i64", 1i64 as usize, "e"),
  ];

  for (type_name, index_value, expected) in test_cases {
    let node = StringCharAtNode::new("test_char_at".to_string());

    let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
    let outputs_future = node.execute(inputs);
    let mut outputs = outputs_future.await.unwrap();

    // Send values: "Hello"[index] -> expected
    let _ = in_tx
      .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
      .await;

    // Send the index in the appropriate type
    let index_arc: Arc<dyn Any + Send + Sync> = match type_name {
      "u32" => Arc::new(index_value as u32),
      "u64" => Arc::new(index_value as u64),
      "i32" => Arc::new(index_value as i32),
      "i64" => Arc::new(index_value as i64),
      _ => Arc::new(index_value),
    };

    let _ = index_tx.send(index_arc).await;

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
    assert_eq!(results[0], expected);
  }
}
