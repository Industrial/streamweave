//! Tests for ArrayFilterNode

use crate::node::{InputStreams, Node};
use crate::nodes::array::ArrayFilterNode;
use crate::nodes::filter_node::{FilterConfig, filter_config};
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
  let (predicate_tx, predicate_rx) = mpsc::channel(10);

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
    "predicate".to_string(),
    Box::pin(ReceiverStream::new(predicate_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, predicate_tx, inputs)
}

#[tokio::test]
async fn test_array_filter_node_creation() {
  let node = ArrayFilterNode::new("test_filter".to_string());
  assert_eq!(node.name(), "test_filter");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("predicate"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_array_filter_basic() {
  let node = ArrayFilterNode::new("test_filter".to_string());

  let (_config_tx, in_tx, predicate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a predicate that filters out values less than 3
  let predicate: FilterConfig = filter_config(|value: Arc<dyn Any + Send + Sync>| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(*arc_i32 >= 3)
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send predicate first
  let _ = predicate_tx
    .send(Arc::new(predicate) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: [1, 2, 3, 4] → [3, 4] (filter >= 3)
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(4i32) as Arc<dyn Any + Send + Sync>,
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
  assert_eq!(results[0].len(), 2);
  // Verify elements are filtered
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  let elem1 = results[0][1].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem0, 3i32);
  assert_eq!(*elem1, 4i32);
}

#[tokio::test]
async fn test_array_filter_empty() {
  let node = ArrayFilterNode::new("test_filter".to_string());

  let (_config_tx, in_tx, predicate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a predicate that filters out nothing
  let predicate: FilterConfig = filter_config(|_value| async move { Ok(true) });

  // Send predicate first
  let _ = predicate_tx
    .send(Arc::new(predicate) as Arc<dyn Any + Send + Sync>)
    .await;

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
async fn test_array_filter_all_filtered() {
  let node = ArrayFilterNode::new("test_filter".to_string());

  let (_config_tx, in_tx, predicate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a predicate that filters out everything
  let predicate: FilterConfig = filter_config(|_value| async move { Ok(false) });

  // Send predicate first
  let _ = predicate_tx
    .send(Arc::new(predicate) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: [1, 2, 3] → [] (all filtered)
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
  assert_eq!(results[0].len(), 0);
}

#[tokio::test]
async fn test_array_filter_strings() {
  let node = ArrayFilterNode::new("test_filter".to_string());

  let (_config_tx, in_tx, predicate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a predicate that filters strings longer than 3 characters
  let predicate: FilterConfig = filter_config(|value: Arc<dyn Any + Send + Sync>| async move {
    if let Ok(arc_str) = value.downcast::<String>() {
      Ok(arc_str.len() > 3)
    } else {
      Err("Expected String".to_string())
    }
  });

  // Send predicate first
  let _ = predicate_tx
    .send(Arc::new(predicate) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: ["a", "ab", "abc", "abcd"] → ["abcd"] (length > 3)
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("ab".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("abc".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("abcd".to_string()) as Arc<dyn Any + Send + Sync>,
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
  let elem0 = results[0][0].clone().downcast::<String>().unwrap();
  assert_eq!(*elem0, "abcd".to_string());
}

#[tokio::test]
async fn test_array_filter_predicate_error() {
  let node = ArrayFilterNode::new("test_filter".to_string());

  let (_config_tx, in_tx, predicate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a predicate that returns an error
  let predicate: FilterConfig = filter_config(|value: Arc<dyn Any + Send + Sync>| async move {
    if let Ok(_arc_i32) = value.downcast::<i32>() {
      Err("Test error".to_string())
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send predicate first
  let _ = predicate_tx
    .send(Arc::new(predicate) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: [1, 2]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("Predicate error"));
}

#[tokio::test]
async fn test_array_filter_non_array_input() {
  let node = ArrayFilterNode::new("test_filter".to_string());

  let (_config_tx, in_tx, predicate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a predicate
  let predicate: FilterConfig = filter_config(|_value| async move { Ok(true) });

  // Send predicate first
  let _ = predicate_tx
    .send(Arc::new(predicate) as Arc<dyn Any + Send + Sync>)
    .await;

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

#[tokio::test]
async fn test_array_filter_invalid_predicate() {
  let node = ArrayFilterNode::new("test_filter".to_string());

  let (_config_tx, _in_tx, predicate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid predicate (not a FilterConfig)
  let _ = predicate_tx
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
  assert!(errors[0].contains("Invalid predicate type"));
}
