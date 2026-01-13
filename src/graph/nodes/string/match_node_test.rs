//! Tests for StringMatchNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::string::StringMatchNode;
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
  let (pattern_tx, pattern_rx) = mpsc::channel(10);

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
    "pattern".to_string(),
    Box::pin(ReceiverStream::new(pattern_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, pattern_tx, inputs)
}

#[tokio::test]
async fn test_string_match_node_creation() {
  let node = StringMatchNode::new("test_match".to_string());
  assert_eq!(node.name(), "test_match");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("pattern"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_string_match_true() {
  let node = StringMatchNode::new("test_match".to_string());

  let (_config_tx, in_tx, pattern_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello123" matches r"\d+" = true (contains digits)
  let _ = in_tx
    .send(Arc::new("Hello123".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = pattern_tx
    .send(Arc::new(r"\d+".to_string()) as Arc<dyn Any + Send + Sync>)
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
async fn test_string_match_false() {
  let node = StringMatchNode::new("test_match".to_string());

  let (_config_tx, in_tx, pattern_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello" matches r"\d+" = false (no digits)
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = pattern_tx
    .send(Arc::new(r"\d+".to_string()) as Arc<dyn Any + Send + Sync>)
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
async fn test_string_match_full_match() {
  let node = StringMatchNode::new("test_match".to_string());

  let (_config_tx, in_tx, pattern_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "123" matches r"^\d+$" = true (full match)
  let _ = in_tx
    .send(Arc::new("123".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = pattern_tx
    .send(Arc::new(r"^\d+$".to_string()) as Arc<dyn Any + Send + Sync>)
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
async fn test_string_match_partial_match() {
  let node = StringMatchNode::new("test_match".to_string());

  let (_config_tx, in_tx, pattern_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello123" matches r"^\d+$" = false (not full match)
  let _ = in_tx
    .send(Arc::new("Hello123".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = pattern_tx
    .send(Arc::new(r"^\d+$".to_string()) as Arc<dyn Any + Send + Sync>)
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
async fn test_string_match_email_pattern() {
  let node = StringMatchNode::new("test_match".to_string());

  let (_config_tx, in_tx, pattern_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "test@example.com" matches email pattern = true
  let _ = in_tx
    .send(Arc::new("test@example.com".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = pattern_tx
    .send(
      Arc::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}".to_string())
        as Arc<dyn Any + Send + Sync>,
    )
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
async fn test_string_match_invalid_regex() {
  let node = StringMatchNode::new("test_match".to_string());

  let (_config_tx, in_tx, pattern_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid regex pattern: "["
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = pattern_tx
    .send(Arc::new("[".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("Invalid regex pattern"));
}

#[tokio::test]
async fn test_string_match_non_string_input() {
  let node = StringMatchNode::new("test_match".to_string());

  let (_config_tx, in_tx, pattern_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-string input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = pattern_tx
    .send(Arc::new(r"\d+".to_string()) as Arc<dyn Any + Send + Sync>)
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
async fn test_string_match_non_string_pattern() {
  let node = StringMatchNode::new("test_match".to_string());

  let (_config_tx, in_tx, pattern_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-string pattern
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = pattern_tx
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
  assert!(errors[0].contains("pattern must be String"));
}
