//! Tests for StringPadNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::string::StringPadNode;
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
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (length_tx, length_rx) = mpsc::channel(10);
  let (padding_tx, padding_rx) = mpsc::channel(10);
  let (side_tx, side_rx) = mpsc::channel(10);

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
    "length".to_string(),
    Box::pin(ReceiverStream::new(length_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "padding".to_string(),
    Box::pin(ReceiverStream::new(padding_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "side".to_string(),
    Box::pin(ReceiverStream::new(side_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, length_tx, padding_tx, side_tx, inputs)
}

#[tokio::test]
async fn test_string_pad_node_creation() {
  let node = StringPadNode::new("test_pad".to_string());
  assert_eq!(node.name(), "test_pad");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("length"));
  assert!(node.has_input_port("padding"));
  assert!(node.has_input_port("side"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_string_pad_right_default() {
  let node = StringPadNode::new("test_pad".to_string());

  let (_config_tx, in_tx, length_tx, padding_tx, side_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello" with length 10, default padding (space), right side = "Hello     "
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = length_tx
    .send(Arc::new(10usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = padding_tx
    .send(Arc::new(" ".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = side_tx
    .send(Arc::new("right".to_string()) as Arc<dyn Any + Send + Sync>)
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
            results.push(arc_str.clone());
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
  assert_eq!(*results[0], "Hello     ");
  assert_eq!(results[0].chars().count(), 10);
}

#[tokio::test]
async fn test_string_pad_left() {
  let node = StringPadNode::new("test_pad".to_string());

  let (_config_tx, in_tx, length_tx, padding_tx, side_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello" with length 10, padding "0", left side = "00000Hello"
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = length_tx
    .send(Arc::new(10usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = padding_tx
    .send(Arc::new("0".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = side_tx
    .send(Arc::new("left".to_string()) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push(arc_str.clone());
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
  assert_eq!(*results[0], "00000Hello");
  assert_eq!(results[0].chars().count(), 10);
}

#[tokio::test]
async fn test_string_pad_center() {
  let node = StringPadNode::new("test_pad".to_string());

  let (_config_tx, in_tx, length_tx, padding_tx, side_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hi" with length 7, padding "-", center side = "--Hi---" (prefer left)
  let _ = in_tx
    .send(Arc::new("Hi".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = length_tx
    .send(Arc::new(7usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = padding_tx
    .send(Arc::new("-".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = side_tx
    .send(Arc::new("center".to_string()) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push(arc_str.clone());
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
  assert_eq!(*results[0], "--Hi---");
  assert_eq!(results[0].chars().count(), 7);
}

#[tokio::test]
async fn test_string_pad_already_longer() {
  let node = StringPadNode::new("test_pad".to_string());

  let (_config_tx, in_tx, length_tx, padding_tx, side_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" (11 chars) with length 5, should return as-is
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = length_tx
    .send(Arc::new(5usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = padding_tx
    .send(Arc::new(" ".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = side_tx
    .send(Arc::new("right".to_string()) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push(arc_str.clone());
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
  assert_eq!(*results[0], "Hello World");
}

#[tokio::test]
async fn test_string_pad_empty_string() {
  let node = StringPadNode::new("test_pad".to_string());

  let (_config_tx, in_tx, length_tx, padding_tx, side_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "" with length 5, padding "x", right side = "xxxxx"
  let _ = in_tx
    .send(Arc::new("".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = length_tx
    .send(Arc::new(5usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = padding_tx
    .send(Arc::new("x".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = side_tx
    .send(Arc::new("right".to_string()) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push(arc_str.clone());
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
  assert_eq!(*results[0], "xxxxx");
  assert_eq!(results[0].chars().count(), 5);
}

#[tokio::test]
async fn test_string_pad_numeric_length_types() {
  let node = StringPadNode::new("test_pad".to_string());

  let (_config_tx, in_tx, length_tx, padding_tx, side_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Test with i32 length
  let _ = in_tx
    .send(Arc::new("Hi".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = length_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = padding_tx
    .send(Arc::new("0".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = side_tx
    .send(Arc::new("right".to_string()) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push(arc_str.clone());
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
  assert_eq!(*results[0], "Hi000");
  assert_eq!(results[0].chars().count(), 5);
}

#[tokio::test]
async fn test_string_pad_invalid_side() {
  let node = StringPadNode::new("test_pad".to_string());

  let (_config_tx, in_tx, length_tx, padding_tx, side_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid side: "invalid"
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = length_tx
    .send(Arc::new(10usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = padding_tx
    .send(Arc::new(" ".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = side_tx
    .send(Arc::new("invalid".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("Unsupported padding side"));
}

#[tokio::test]
async fn test_string_pad_negative_length() {
  let node = StringPadNode::new("test_pad".to_string());

  let (_config_tx, in_tx, length_tx, padding_tx, side_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send negative length
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = length_tx
    .send(Arc::new(-5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = padding_tx
    .send(Arc::new(" ".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = side_tx
    .send(Arc::new("right".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("cannot be negative"));
}

#[tokio::test]
async fn test_string_pad_non_string_input() {
  let node = StringPadNode::new("test_pad".to_string());

  let (_config_tx, in_tx, length_tx, padding_tx, side_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-string input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = length_tx
    .send(Arc::new(10usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = padding_tx
    .send(Arc::new(" ".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = side_tx
    .send(Arc::new("right".to_string()) as Arc<dyn Any + Send + Sync>)
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
