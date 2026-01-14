//! Tests for ReduceNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::reduction::{
  reduce_config, ReduceConfig, ReduceConfigWrapper, ReduceNode,
};
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
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (initial_tx, initial_rx) = mpsc::channel(10);
  let (function_tx, function_rx) = mpsc::channel(10);

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
    "initial".to_string(),
    Box::pin(ReceiverStream::new(initial_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "function".to_string(),
    Box::pin(ReceiverStream::new(function_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, initial_tx, function_tx, inputs)
}

#[tokio::test]
async fn test_reduce_node_creation() {
  let node = ReduceNode::new("test_reduce".to_string());
  assert_eq!(node.name(), "test_reduce");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("initial"));
  assert!(node.has_input_port("function"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_reduce_sum() {
  let node = ReduceNode::new("test_reduce".to_string());

  let (_config_tx, in_tx, initial_tx, function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a sum reduction function
  let sum_function: ReduceConfig = reduce_config(|acc, value| async move {
    if let (Ok(acc_i32), Ok(val_i32)) = (acc.downcast::<i32>(), value.downcast::<i32>()) {
      Ok(Arc::new(*acc_i32 + *val_i32) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send initial value
  let _ = initial_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send function
  let _ = function_tx
    .send(Arc::new(ReduceConfigWrapper::new(sum_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: 1, 2, 3 → sum = 6 (0 + 1 + 2 + 3)
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx); // Close the input stream
  drop(initial_tx); // Close the initial stream
  drop(function_tx); // Close the function stream

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

  assert_eq!(results.len(), 1);
  if let Ok(sum) = results[0].clone().downcast::<i32>() {
    assert_eq!(*sum, 6i32);
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
async fn test_reduce_empty_stream() {
  let node = ReduceNode::new("test_reduce".to_string());

  let (_config_tx, in_tx, initial_tx, function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a sum reduction function
  let sum_function: ReduceConfig = reduce_config(|acc, value| async move {
    if let (Ok(acc_i32), Ok(val_i32)) = (acc.downcast::<i32>(), value.downcast::<i32>()) {
      Ok(Arc::new(*acc_i32 + *val_i32) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send initial value
  let _ = initial_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send function
  let _ = function_tx
    .send(Arc::new(ReduceConfigWrapper::new(sum_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send no values: empty stream → result should be initial value
  drop(in_tx); // Close the input stream immediately
  drop(initial_tx); // Close the initial stream
  drop(function_tx); // Close the function stream

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

  assert_eq!(results.len(), 1);
  if let Ok(result) = results[0].clone().downcast::<i32>() {
    assert_eq!(*result, 0i32); // Should be the initial value
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
async fn test_reduce_single_value() {
  let node = ReduceNode::new("test_reduce".to_string());

  let (_config_tx, in_tx, initial_tx, function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a sum reduction function
  let sum_function: ReduceConfig = reduce_config(|acc, value| async move {
    if let (Ok(acc_i32), Ok(val_i32)) = (acc.downcast::<i32>(), value.downcast::<i32>()) {
      Ok(Arc::new(*acc_i32 + *val_i32) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send initial value
  let _ = initial_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send function
  let _ = function_tx
    .send(Arc::new(ReduceConfigWrapper::new(sum_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send single value: 5 → result = 15 (10 + 5)
  let _ = in_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx); // Close the input stream
  drop(initial_tx); // Close the initial stream
  drop(function_tx); // Close the function stream

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

  assert_eq!(results.len(), 1);
  if let Ok(result) = results[0].clone().downcast::<i32>() {
    assert_eq!(*result, 15i32);
  } else {
    panic!("Result is not an i32");
  }
}
