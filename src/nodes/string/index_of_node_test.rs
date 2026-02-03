//! Tests for StringIndexOfNode

use crate::node::{InputStreams, Node};
use crate::nodes::string::StringIndexOfNode;
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
  let (substring_tx, substring_rx) = mpsc::channel(10);

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
    "substring".to_string(),
    Box::pin(ReceiverStream::new(substring_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, substring_tx, inputs)
}

#[tokio::test]
async fn test_string_index_of_node_creation() {
  let node = StringIndexOfNode::new("test_index_of".to_string());
  assert_eq!(node.name(), "test_index_of");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("substring"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_string_index_of_found() {
  let node = StringIndexOfNode::new("test_index_of".to_string());

  let (_config_tx, in_tx, substring_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" index of "World" = 6
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = substring_tx
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
          if let Ok(arc_i64) = item.downcast::<i64>() {
            results.push(*arc_i64);
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
  assert_eq!(results[0], 6);
}

#[tokio::test]
async fn test_string_index_of_not_found() {
  let node = StringIndexOfNode::new("test_index_of".to_string());

  let (_config_tx, in_tx, substring_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" index of "Universe" = -1
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = substring_tx
    .send(Arc::new("Universe".to_string()) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i64) = item.downcast::<i64>() {
            results.push(*arc_i64);
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
  assert_eq!(results[0], -1);
}

#[tokio::test]
async fn test_string_index_of_case_sensitive() {
  let node = StringIndexOfNode::new("test_index_of".to_string());

  let (_config_tx, in_tx, substring_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" index of "world" = -1 (case sensitive)
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = substring_tx
    .send(Arc::new("world".to_string()) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i64) = item.downcast::<i64>() {
            results.push(*arc_i64);
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
  assert_eq!(results[0], -1);
}

#[tokio::test]
async fn test_string_index_of_at_start() {
  let node = StringIndexOfNode::new("test_index_of".to_string());

  let (_config_tx, in_tx, substring_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" index of "Hello" = 0
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = substring_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i64) = item.downcast::<i64>() {
            results.push(*arc_i64);
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
  assert_eq!(results[0], 0);
}

#[tokio::test]
async fn test_string_index_of_empty_substring() {
  let node = StringIndexOfNode::new("test_index_of".to_string());

  let (_config_tx, in_tx, substring_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello" index of "" = 0 (empty string is found at start)
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = substring_tx
    .send(Arc::new("".to_string()) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i64) = item.downcast::<i64>() {
            results.push(*arc_i64);
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
  assert_eq!(results[0], 0);
}
