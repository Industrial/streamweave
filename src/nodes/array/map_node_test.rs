//! Tests for ArrayMapNode

use crate::node::{InputStreams, Node};
use crate::nodes::array::ArrayMapNode;
use crate::nodes::map_node::{MapConfig, map_config};
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
  let (function_tx, function_rx) = mpsc::channel(10);

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
    "function".to_string(),
    Box::pin(ReceiverStream::new(function_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, function_tx, inputs)
}

#[tokio::test]
async fn test_array_map_node_creation() {
  let node = ArrayMapNode::new("test_map".to_string());
  assert_eq!(node.name(), "test_map");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("function"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_array_map_basic() {
  let node = ArrayMapNode::new("test_map".to_string());

  let (_config_tx, in_tx, function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a function that multiplies by 2
  let map_fn: MapConfig = map_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send function first
  let _ = function_tx
    .send(Arc::new(map_fn) as Arc<dyn Any + Send + Sync>)
    .await;

  // Wait a bit for function to be processed
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send values: [1, 2, 3, 4] → [2, 4, 6, 8] (multiply by 2)
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
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
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
  assert_eq!(results[0].len(), 4);
  // Verify elements are mapped
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  let elem1 = results[0][1].clone().downcast::<i32>().unwrap();
  let elem2 = results[0][2].clone().downcast::<i32>().unwrap();
  let elem3 = results[0][3].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem0, 2i32);
  assert_eq!(*elem1, 4i32);
  assert_eq!(*elem2, 6i32);
  assert_eq!(*elem3, 8i32);
}

#[tokio::test]
async fn test_array_map_empty() {
  let node = ArrayMapNode::new("test_map".to_string());

  let (_config_tx, in_tx, function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a function that multiplies by 2
  let map_fn: MapConfig = map_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send function first
  let _ = function_tx
    .send(Arc::new(map_fn) as Arc<dyn Any + Send + Sync>)
    .await;

  // Wait a bit for function to be processed
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send values: [] → []
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
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
async fn test_array_map_strings() {
  let node = ArrayMapNode::new("test_map".to_string());

  let (_config_tx, in_tx, function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a function that converts strings to uppercase
  let map_fn: MapConfig = map_config(|value| async move {
    if let Ok(arc_str) = value.downcast::<String>() {
      Ok(Arc::new(arc_str.to_uppercase()) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected String".to_string())
    }
  });

  // Send function first
  let _ = function_tx
    .send(Arc::new(map_fn) as Arc<dyn Any + Send + Sync>)
    .await;

  // Wait a bit for function to be processed
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send values: ["a", "b", "c"] → ["A", "B", "C"]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("b".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("c".to_string()) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
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
  let elem0 = results[0][0].clone().downcast::<String>().unwrap();
  let elem1 = results[0][1].clone().downcast::<String>().unwrap();
  let elem2 = results[0][2].clone().downcast::<String>().unwrap();
  assert_eq!(*elem0, "A".to_string());
  assert_eq!(*elem1, "B".to_string());
  assert_eq!(*elem2, "C".to_string());
}

#[tokio::test]
async fn test_array_map_type_change() {
  let node = ArrayMapNode::new("test_map".to_string());

  let (_config_tx, in_tx, function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a function that converts i32 to String
  let map_fn: MapConfig = map_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(Arc::new(arc_i32.to_string()) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send function first
  let _ = function_tx
    .send(Arc::new(map_fn) as Arc<dyn Any + Send + Sync>)
    .await;

  // Wait a bit for function to be processed
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send values: [1, 2, 3] → ["1", "2", "3"]
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
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
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
  let elem0 = results[0][0].clone().downcast::<String>().unwrap();
  let elem1 = results[0][1].clone().downcast::<String>().unwrap();
  let elem2 = results[0][2].clone().downcast::<String>().unwrap();
  assert_eq!(*elem0, "1".to_string());
  assert_eq!(*elem1, "2".to_string());
  assert_eq!(*elem2, "3".to_string());
}

#[tokio::test]
async fn test_array_map_function_error() {
  let node = ArrayMapNode::new("test_map".to_string());

  let (_config_tx, in_tx, function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a function that returns an error
  let map_fn: MapConfig = map_config(|value| async move {
    if let Ok(_arc_i32) = value.downcast::<i32>() {
      Err("Test error".to_string())
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send function first
  let _ = function_tx
    .send(Arc::new(map_fn) as Arc<dyn Any + Send + Sync>)
    .await;

  // Wait a bit for function to be processed
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

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
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
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
  assert!(errors[0].contains("Map function error"));
}

#[tokio::test]
async fn test_array_map_non_array_input() {
  let node = ArrayMapNode::new("test_map".to_string());

  let (_config_tx, in_tx, function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a function
  let map_fn: MapConfig = map_config(|value| async move {
    Ok(value) // Identity function
  });

  // Send function first
  let _ = function_tx
    .send(Arc::new(map_fn) as Arc<dyn Any + Send + Sync>)
    .await;

  // Wait a bit for function to be processed
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send non-array input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
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
async fn test_array_map_invalid_function() {
  let node = ArrayMapNode::new("test_map".to_string());

  let (_config_tx, _in_tx, function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid function (not a MapConfig)
  let _ = function_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Wait a bit for error to be processed
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
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
  assert!(errors[0].contains("Invalid configuration type"));
}
