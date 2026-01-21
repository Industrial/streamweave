#![allow(unused_imports, dead_code, unused, clippy::type_complexity)]

use super::WindowNode;
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
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (size_tx, size_rx) = mpsc::channel(10);

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
    "size".to_string(),
    Box::pin(ReceiverStream::new(size_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, size_tx, inputs)
}

#[tokio::test]
async fn test_window_node_creation() {
  let node = WindowNode::new("test_window".to_string());
  assert_eq!(node.name(), "test_window");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("size"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_window_node_exact_windows() {
  let node = WindowNode::new("test_window".to_string());

  let (_config_tx, in_tx, size_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Set window size: 3
  let window_size = 3usize;
  size_tx
    .send(Arc::new(window_size) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send 9 items: should produce 3 windows of 3 items each
  let test_values = vec![1i32, 2i32, 3i32, 4i32, 5i32, 6i32, 7i32, 8i32, 9i32];
  for value in test_values {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream
  drop(size_tx); // Close the size stream

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(300));
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

  // Should have received 3 windows
  assert_eq!(results.len(), 3);

  // Check each window
  let expected_windows = vec![
    vec![1i32, 2i32, 3i32],
    vec![4i32, 5i32, 6i32],
    vec![7i32, 8i32, 9i32],
  ];

  for (i, result) in results.iter().enumerate() {
    if let Ok(window) = result.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
      assert_eq!(window.len(), window_size);
      for (j, item) in window.iter().enumerate() {
        if let Ok(value) = item.clone().downcast::<i32>() {
          assert_eq!(*value, expected_windows[i][j]);
        } else {
          panic!("Window item at [{}, {}] is not an i32", i, j);
        }
      }
    } else {
      panic!("Result at index {} is not a Vec", i);
    }
  }
}

#[tokio::test]
async fn test_window_node_partial_window_discarded() {
  let node = WindowNode::new("test_window".to_string());

  let (_config_tx, in_tx, size_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Set window size: 3
  let window_size = 3usize;
  size_tx
    .send(Arc::new(window_size) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send 7 items: should produce 2 complete windows, partial window discarded
  let test_values = vec![1i32, 2i32, 3i32, 4i32, 5i32, 6i32, 7i32];
  for value in test_values {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream
  drop(size_tx); // Close the size stream

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(300));
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

  // Should have received 2 complete windows (partial discarded)
  assert_eq!(results.len(), 2);

  // Check the windows
  let expected_windows = vec![vec![1i32, 2i32, 3i32], vec![4i32, 5i32, 6i32]];

  for (i, result) in results.iter().enumerate() {
    if let Ok(window) = result.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
      assert_eq!(window.len(), window_size);
      for (j, item) in window.iter().enumerate() {
        if let Ok(value) = item.clone().downcast::<i32>() {
          assert_eq!(*value, expected_windows[i][j]);
        } else {
          panic!("Window item at [{}, {}] is not an i32", i, j);
        }
      }
    } else {
      panic!("Result at index {} is not a Vec", i);
    }
  }
}

#[tokio::test]
async fn test_window_node_empty_stream() {
  let node = WindowNode::new("test_window".to_string());

  let (_config_tx, in_tx, size_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Set window size but send no items
  let window_size = 3usize;
  size_tx
    .send(Arc::new(window_size) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Close input streams
  drop(in_tx);
  drop(size_tx);

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

  // Should have received no windows
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_window_node_error_handling() {
  let node = WindowNode::new("test_window".to_string());

  let (_config_tx, in_tx, size_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send invalid window size (negative number)
  let invalid_size = -1i32;
  size_tx
    .send(Arc::new(invalid_size) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Close input streams
  drop(in_tx);
  drop(size_tx);

  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          errors.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have received an error about negative window size
  assert_eq!(errors.len(), 1);
  if let Ok(error_msg) = errors[0].clone().downcast::<String>() {
    assert!(error_msg.contains("cannot be negative"));
  } else {
    panic!("Error result is not a String");
  }
}
