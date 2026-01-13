//! Tests for StringStartsWithNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::string::StringStartsWithNode;
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
  let (in_tx, in_rx) = mpsc::channel(10);
  let (prefix_tx, prefix_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "prefix".to_string(),
    Box::pin(ReceiverStream::new(prefix_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, prefix_tx, inputs)
}

#[tokio::test]
async fn test_string_starts_with_node_creation() {
  let node = StringStartsWithNode::new("test_starts_with".to_string());
  assert_eq!(node.name(), "test_starts_with");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("prefix"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_string_starts_with_true() {
  let node = StringStartsWithNode::new("test_starts_with".to_string());

  let (_config_tx, in_tx, prefix_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" starts with "Hello" = true
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = prefix_tx
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}

#[tokio::test]
async fn test_string_starts_with_false() {
  let node = StringStartsWithNode::new("test_starts_with".to_string());

  let (_config_tx, in_tx, prefix_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" starts with "World" = false
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = prefix_tx
    .send(Arc::new("World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(!results[0]);
}

#[tokio::test]
async fn test_string_starts_with_case_sensitive() {
  let node = StringStartsWithNode::new("test_starts_with".to_string());

  let (_config_tx, in_tx, prefix_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" starts with "hello" = false (case-sensitive)
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = prefix_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(!results[0]);
}

#[tokio::test]
async fn test_string_starts_with_empty_prefix() {
  let node = StringStartsWithNode::new("test_starts_with".to_string());

  let (_config_tx, in_tx, prefix_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello" starts with "" = true (empty string is always a prefix)
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = prefix_tx
    .send(Arc::new("".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}

#[tokio::test]
async fn test_string_starts_with_empty_string() {
  let node = StringStartsWithNode::new("test_starts_with".to_string());

  let (_config_tx, in_tx, prefix_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "" starts with "x" = false
  let _ = in_tx
    .send(Arc::new("".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = prefix_tx
    .send(Arc::new("x".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(!results[0]);
}

#[tokio::test]
async fn test_string_starts_with_exact_match() {
  let node = StringStartsWithNode::new("test_starts_with".to_string());

  let (_config_tx, in_tx, prefix_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello" starts with "Hello" = true (exact match)
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = prefix_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}

#[tokio::test]
async fn test_string_starts_with_non_string_input() {
  let node = StringStartsWithNode::new("test_starts_with".to_string());

  let (_config_tx, in_tx, prefix_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-string input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = prefix_tx
    .send(Arc::new("x".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            errors.push(arc_str.clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("input must be String"));
}

#[tokio::test]
async fn test_string_starts_with_non_string_prefix() {
  let node = StringStartsWithNode::new("test_starts_with".to_string());

  let (_config_tx, in_tx, prefix_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-string prefix
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = prefix_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            errors.push(arc_str.clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("prefix must be String"));
}
