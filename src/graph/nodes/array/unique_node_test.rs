//! Tests for ArrayUniqueNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::array::ArrayUniqueNode;
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
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, inputs)
}

#[tokio::test]
async fn test_array_unique_node_creation() {
  let node = ArrayUniqueNode::new("test_unique".to_string());
  assert_eq!(node.name(), "test_unique");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_array_unique_basic() {
  let node = ArrayUniqueNode::new("test_unique".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 2, 3, 1] → [1, 2, 3]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_vec) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
            results.push(arc_vec.clone());
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
  assert_eq!(results[0].len(), 3);
  // Verify elements are unique and in order
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  let elem1 = results[0][1].clone().downcast::<i32>().unwrap();
  let elem2 = results[0][2].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem0, 1i32);
  assert_eq!(*elem1, 2i32);
  assert_eq!(*elem2, 3i32);
}

#[tokio::test]
async fn test_array_unique_empty() {
  let node = ArrayUniqueNode::new("test_unique".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [] → []
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_vec) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
            results.push(arc_vec.clone());
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
  assert_eq!(results[0].len(), 0);
}

#[tokio::test]
async fn test_array_unique_all_unique() {
  let node = ArrayUniqueNode::new("test_unique".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3] → [1, 2, 3] (all unique, no change)
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_vec) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
            results.push(arc_vec.clone());
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
  assert_eq!(results[0].len(), 3);
}

#[tokio::test]
async fn test_array_unique_all_duplicates() {
  let node = ArrayUniqueNode::new("test_unique".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 1, 1, 1] → [1] (all duplicates, only first kept)
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_vec) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
            results.push(arc_vec.clone());
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
  assert_eq!(results[0].len(), 1);
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem0, 1i32);
}

#[tokio::test]
async fn test_array_unique_strings() {
  let node = ArrayUniqueNode::new("test_unique".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: ["a", "b", "a", "c", "b"] → ["a", "b", "c"]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("b".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("c".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("b".to_string()) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_vec) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
            results.push(arc_vec.clone());
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
  assert_eq!(results[0].len(), 3);
  // Verify elements are unique and in order
  let elem0 = results[0][0].clone().downcast::<String>().unwrap();
  let elem1 = results[0][1].clone().downcast::<String>().unwrap();
  let elem2 = results[0][2].clone().downcast::<String>().unwrap();
  assert_eq!(*elem0, "a".to_string());
  assert_eq!(*elem1, "b".to_string());
  assert_eq!(*elem2, "c".to_string());
}

#[tokio::test]
async fn test_array_unique_mixed_types() {
  let node = ArrayUniqueNode::new("test_unique".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, "a", 1, "b", "a"] → [1, "a", "b"] (different types are not duplicates)
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new("b".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_vec) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
            results.push(arc_vec.clone());
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
  assert_eq!(results[0].len(), 3);
  // Verify elements are unique (1, "a", "b")
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  let elem1 = results[0][1].clone().downcast::<String>().unwrap();
  let elem2 = results[0][2].clone().downcast::<String>().unwrap();
  assert_eq!(*elem0, 1i32);
  assert_eq!(*elem1, "a".to_string());
  assert_eq!(*elem2, "b".to_string());
}

#[tokio::test]
async fn test_array_unique_non_array_input() {
  let node = ArrayUniqueNode::new("test_unique".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-array input
  let _ = in_tx
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
  assert!(errors[0].contains("input must be Vec"));
}
