#![allow(unused_imports, dead_code, unused, clippy::type_complexity)]

use super::DistinctUntilChangedNode;
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
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, inputs)
}

#[tokio::test]
async fn test_distinct_until_changed_node_creation() {
  let node = DistinctUntilChangedNode::new("test_distinct_until_changed".to_string());
  assert_eq!(node.name(), "test_distinct_until_changed");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_distinct_until_changed_consecutive_duplicates() {
  let node = DistinctUntilChangedNode::new("test_distinct_until_changed".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send items with consecutive duplicates: A, A, B, B, A, C, C
  // Expected output: A, B, A, C (only when value changes)
  let test_values = vec!["A", "A", "B", "B", "A", "C", "C"];
  let expected_unique = vec!["A", "B", "A", "C"];

  for value in test_values {
    in_tx
      .send(Arc::new(value.to_string()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream

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

  // Should have received only items that differed from the previous emission
  assert_eq!(results.len(), 4);

  // Check each result
  for (i, result) in results.iter().enumerate() {
    if let Ok(value) = result.clone().downcast::<String>() {
      assert_eq!(*value, expected_unique[i]);
    } else {
      panic!("Result at index {} is not a String", i);
    }
  }
}

#[tokio::test]
async fn test_distinct_until_changed_all_unique() {
  let node = DistinctUntilChangedNode::new("test_distinct_until_changed".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send all unique items: 1, 2, 3, 4, 5
  // Expected output: all items (no consecutive duplicates)
  let test_values = vec![1i32, 2i32, 3i32, 4i32, 5i32];

  for value in test_values.clone() {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream

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

  // Should have received all items (all are unique from previous)
  assert_eq!(results.len(), 5);

  // Check each result
  for (i, result) in results.iter().enumerate() {
    if let Ok(value) = result.clone().downcast::<i32>() {
      assert_eq!(*value, test_values[i]);
    } else {
      panic!("Result at index {} is not an i32", i);
    }
  }
}

#[tokio::test]
async fn test_distinct_until_changed_empty_stream() {
  let node = DistinctUntilChangedNode::new("test_distinct_until_changed".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send no items (empty stream)
  drop(in_tx); // Close the input stream immediately

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

  // Should have received no items (empty stream)
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_distinct_until_changed_single_item() {
  let node = DistinctUntilChangedNode::new("test_distinct_until_changed".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send single item
  let test_value = 42i32;
  in_tx
    .send(Arc::new(test_value) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  drop(in_tx); // Close the input stream

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

  // Should have received the single item
  assert_eq!(results.len(), 1);
  if let Ok(value) = results[0].clone().downcast::<i32>() {
    assert_eq!(*value, test_value);
  } else {
    panic!("Result is not an i32");
  }
}
