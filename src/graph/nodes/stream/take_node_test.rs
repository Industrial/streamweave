//! Tests for TakeNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::stream::TakeNode;
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
  let (count_tx, count_rx) = mpsc::channel(10);

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
    "count".to_string(),
    Box::pin(ReceiverStream::new(count_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, count_tx, inputs)
}

#[tokio::test]
async fn test_take_node_creation() {
  let node = TakeNode::new("test_take".to_string());
  assert_eq!(node.name(), "test_take");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("count"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_take_first_n_items() {
  let node = TakeNode::new("test_take".to_string());

  let (_config_tx, in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send count: 3
  let _ = count_tx
    .send(Arc::new(3usize) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send 5 items
  for i in 1..=5 {
    let _ = in_tx
      .send(Arc::new(i) as Arc<dyn Any + Send + Sync>)
      .await;
  }
  drop(in_tx);
  drop(count_tx);

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
          if results.len() == 3 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 3);
  // Verify we got the first 3 items
  if let (Ok(val1), Ok(val2), Ok(val3)) = (
    results[0].clone().downcast::<i32>(),
    results[1].clone().downcast::<i32>(),
    results[2].clone().downcast::<i32>(),
  ) {
    assert_eq!(*val1, 1);
    assert_eq!(*val2, 2);
    assert_eq!(*val3, 3);
  } else {
    panic!("Results are not i32");
  }
}

#[tokio::test]
async fn test_take_zero_items() {
  let node = TakeNode::new("test_take".to_string());

  let (_config_tx, in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send count: 0
  let _ = count_tx
    .send(Arc::new(0usize) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send 3 items
  for i in 1..=3 {
    let _ = in_tx
      .send(Arc::new(i) as Arc<dyn Any + Send + Sync>)
      .await;
  }
  drop(in_tx);
  drop(count_tx);

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

  // Should have taken 0 items
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_take_more_than_available() {
  let node = TakeNode::new("test_take".to_string());

  let (_config_tx, in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send count: 10
  let _ = count_tx
    .send(Arc::new(10usize) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send only 3 items
  for i in 1..=3 {
    let _ = in_tx
      .send(Arc::new(i) as Arc<dyn Any + Send + Sync>)
      .await;
  }
  drop(in_tx);
  drop(count_tx);

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

  // Should have taken all 3 available items
  assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_take_i32_count() {
  let node = TakeNode::new("test_take".to_string());

  let (_config_tx, in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send count: 2 as i32
  let _ = count_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send 5 items
  for i in 1..=5 {
    let _ = in_tx
      .send(Arc::new(i) as Arc<dyn Any + Send + Sync>)
      .await;
  }
  drop(in_tx);
  drop(count_tx);

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
          if results.len() == 2 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_take_invalid_count_negative() {
  let node = TakeNode::new("test_take".to_string());

  let (_config_tx, in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid count: negative value
  let _ = count_tx
    .send(Arc::new(-5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(count_tx);
  drop(in_tx);

  // Check error output
  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<String> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            errors.push((*arc_str).clone());
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("negative"));
}

#[tokio::test]
async fn test_take_invalid_count_type() {
  let node = TakeNode::new("test_take".to_string());

  let (_config_tx, in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid count: string instead of numeric
  let _ = count_tx
    .send(Arc::new("invalid".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(count_tx);
  drop(in_tx);

  // Check error output
  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<String> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            errors.push((*arc_str).clone());
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("numeric") || errors[0].contains("Unsupported"));
}

