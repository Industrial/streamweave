//! Tests for ArrayIndexNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::array::ArrayIndexNode;
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
  let (index_tx, index_rx) = mpsc::channel(10);

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
    "index".to_string(),
    Box::pin(ReceiverStream::new(index_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, index_tx, inputs)
}

#[tokio::test]
async fn test_array_index_node_creation() {
  let node = ArrayIndexNode::new("test_index".to_string());
  assert_eq!(node.name(), "test_index");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("index"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_array_index_basic() {
  let node = ArrayIndexNode::new("test_index".to_string());

  let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3] with index 1 → 2
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i32) = item.downcast::<i32>() {
            results.push(arc_i32.clone());
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
  assert_eq!(*results[0], 2i32);
}

#[tokio::test]
async fn test_array_index_first_element() {
  let node = ArrayIndexNode::new("test_index".to_string());

  let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: ["hello", "world"] with index 0 → "hello"
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("world".to_string()) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = index_tx
    .send(Arc::new(0usize) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(*results[0], "hello".to_string());
}

#[tokio::test]
async fn test_array_index_last_element() {
  let node = ArrayIndexNode::new("test_index".to_string());

  let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3] with index 2 → 3
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = index_tx
    .send(Arc::new(2usize) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i32) = item.downcast::<i32>() {
            results.push(arc_i32.clone());
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
  assert_eq!(*results[0], 3i32);
}

#[tokio::test]
async fn test_array_index_out_of_bounds() {
  let node = ArrayIndexNode::new("test_index".to_string());

  let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2] with index 5 → error
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = index_tx
    .send(Arc::new(5usize) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("out of bounds"));
}

#[tokio::test]
async fn test_array_index_numeric_index_types() {
  let node = ArrayIndexNode::new("test_index".to_string());

  let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [10, 20, 30] with index 1 (as i32) → 20
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(10i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(20i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(30i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = index_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i32) = item.downcast::<i32>() {
            results.push(arc_i32.clone());
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
  assert_eq!(*results[0], 20i32);
}

#[tokio::test]
async fn test_array_index_negative_index() {
  let node = ArrayIndexNode::new("test_index".to_string());

  let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2] with index -1 → error
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = index_tx
    .send(Arc::new(-1i32) as Arc<dyn Any + Send + Sync>)
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
async fn test_array_index_non_array_input() {
  let node = ArrayIndexNode::new("test_index".to_string());

  let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-array input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = index_tx
    .send(Arc::new(0usize) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("input must be Vec"));
}

#[tokio::test]
async fn test_array_index_non_numeric_index() {
  let node = ArrayIndexNode::new("test_index".to_string());

  let (_config_tx, in_tx, index_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-numeric index
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![Arc::new(1i32) as Arc<dyn Any + Send + Sync>];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = index_tx
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
  assert!(errors[0].contains("must be numeric"));
}
