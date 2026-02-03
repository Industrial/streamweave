#![allow(unused_imports, dead_code, unused, clippy::type_complexity)]

use super::{ScanConfig, ScanConfigWrapper, ScanNode, scan_config};
use crate::node::{InputStreams, Node, OutputStreams};
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
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "initial".to_string(),
    Box::pin(ReceiverStream::new(initial_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "function".to_string(),
    Box::pin(ReceiverStream::new(function_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, initial_tx, function_tx, inputs)
}

#[tokio::test]
async fn test_scan_node_creation() {
  let node = ScanNode::new("test_scan".to_string());
  assert_eq!(node.name(), "test_scan");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("initial"));
  assert!(node.has_input_port("function"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_scan_sum() {
  let node = ScanNode::new("test_scan".to_string());

  let (_config_tx, in_tx, initial_tx, function_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Create a sum reduction function
  let sum_function: ScanConfig = scan_config(
    |acc: Arc<dyn Any + Send + Sync>, value: Arc<dyn Any + Send + Sync>| async move {
      if let (Ok(acc_i32), Ok(val_i32)) = (acc.downcast::<i32>(), value.downcast::<i32>()) {
        Ok(Arc::new(*acc_i32 + *val_i32) as Arc<dyn Any + Send + Sync>)
      } else {
        Err("Expected i32".to_string())
      }
    },
  );

  // Send initial value
  let _ = initial_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send function
  let _ = function_tx
    .send(Arc::new(ScanConfigWrapper::new(sum_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: 1, 2, 3
  // Expected scan results: 0 (initial), 1 (0+1), 3 (1+2), 6 (3+3)
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

  // Should have 4 results: initial value + 3 accumulated values
  assert_eq!(results.len(), 4);

  // Check each result
  let expected = [0i32, 1i32, 3i32, 6i32];
  for (i, result) in results.iter().enumerate() {
    if let Ok(value) = result.clone().downcast::<i32>() {
      assert_eq!(
        *value, expected[i],
        "Result at index {} should be {}",
        i, expected[i]
      );
    } else {
      panic!("Result at index {} is not an i32", i);
    }
  }
}

#[tokio::test]
async fn test_scan_empty_stream() {
  let node = ScanNode::new("test_scan".to_string());

  let (_config_tx, in_tx, initial_tx, function_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Create a sum reduction function
  let sum_function: ScanConfig = scan_config(
    |acc: Arc<dyn Any + Send + Sync>, value: Arc<dyn Any + Send + Sync>| async move {
      if let (Ok(acc_i32), Ok(val_i32)) = (acc.downcast::<i32>(), value.downcast::<i32>()) {
        Ok(Arc::new(*acc_i32 + *val_i32) as Arc<dyn Any + Send + Sync>)
      } else {
        Err("Expected i32".to_string())
      }
    },
  );

  // Send initial value
  let _ = initial_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send function
  let _ = function_tx
    .send(Arc::new(ScanConfigWrapper::new(sum_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send no values: empty stream → result should be just initial value
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

  // Should have only the initial value
  assert_eq!(results.len(), 1);
  if let Ok(result) = results[0].clone().downcast::<i32>() {
    assert_eq!(*result, 0i32); // Should be the initial value
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
async fn test_scan_single_value() {
  let node = ScanNode::new("test_scan".to_string());

  let (_config_tx, in_tx, initial_tx, function_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Create a sum reduction function
  let sum_function: ScanConfig = scan_config(
    |acc: Arc<dyn Any + Send + Sync>, value: Arc<dyn Any + Send + Sync>| async move {
      if let (Ok(acc_i32), Ok(val_i32)) = (acc.downcast::<i32>(), value.downcast::<i32>()) {
        Ok(Arc::new(*acc_i32 + *val_i32) as Arc<dyn Any + Send + Sync>)
      } else {
        Err("Expected i32".to_string())
      }
    },
  );

  // Send initial value
  let _ = initial_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send function
  let _ = function_tx
    .send(Arc::new(ScanConfigWrapper::new(sum_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send single value: 5
  // Expected scan results: 10 (initial), 15 (10+5)
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

  // Should have 2 results: initial value + 1 accumulated value
  assert_eq!(results.len(), 2);

  // Check each result
  let expected = [10i32, 15i32];
  for (i, result) in results.iter().enumerate() {
    if let Ok(value) = result.clone().downcast::<i32>() {
      assert_eq!(
        *value, expected[i],
        "Result at index {} should be {}",
        i, expected[i]
      );
    } else {
      panic!("Result at index {} is not an i32", i);
    }
  }
}
