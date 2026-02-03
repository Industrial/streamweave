#![allow(unused_imports, dead_code, unused, clippy::type_complexity)]

use super::{FilterMapConfig, FilterMapConfigWrapper, FilterMapNode, filter_map_config};
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
async fn test_filter_map_node_creation() {
  let node = FilterMapNode::new("test_filter_map".to_string());
  assert_eq!(node.name(), "test_filter_map");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("function"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_filter_map_filter_and_double() {
  let node = FilterMapNode::new("test_filter_map".to_string());

  let (_config_tx, in_tx, function_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Create a filter-map function that filters even numbers and doubles them
  let filter_double_function: FilterMapConfig = filter_map_config(|value| async move {
    if let Ok(i32_val) = value.downcast::<i32>() {
      if *i32_val % 2 == 0 {
        // Even number: double it
        Ok(Some(Arc::new(*i32_val * 2) as Arc<dyn Any + Send + Sync>))
      } else {
        // Odd number: filter out
        Ok(None)
      }
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send function
  let _ = function_tx
    .send(
      Arc::new(FilterMapConfigWrapper::new(filter_double_function)) as Arc<dyn Any + Send + Sync>,
    )
    .await;

  // Send values: 1, 2, 3, 4, 5, 6
  // Expected output: 4, 8, 12 (doubled evens: 2->4, 4->8, 6->12; odds filtered)
  let test_values = vec![1i32, 2i32, 3i32, 4i32, 5i32, 6i32];
  let expected_output = [4i32, 8i32, 12i32];

  for value in test_values {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream
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

  // Should have received 3 items (filtered evens, doubled)
  assert_eq!(results.len(), 3);

  // Check each result
  for (i, result) in results.iter().enumerate() {
    if let Ok(value) = result.clone().downcast::<i32>() {
      assert_eq!(*value, expected_output[i]);
    } else {
      panic!("Result at index {} is not an i32", i);
    }
  }
}

#[tokio::test]
async fn test_filter_map_all_filtered() {
  let node = FilterMapNode::new("test_filter_map".to_string());

  let (_config_tx, in_tx, function_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Create a function that filters everything
  let filter_all_function: FilterMapConfig = filter_map_config(|_value| async move {
    Ok(None) // Filter out everything
  });

  // Send function
  let _ = function_tx
    .send(Arc::new(FilterMapConfigWrapper::new(filter_all_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: 1, 2, 3
  let test_values = vec![1i32, 2i32, 3i32];

  for value in test_values {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream
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

  // Should have received no items (all filtered)
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_filter_map_passthrough() {
  let node = FilterMapNode::new("test_filter_map".to_string());

  let (_config_tx, in_tx, function_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Create a function that passes everything through unchanged
  let passthrough_function: FilterMapConfig = filter_map_config(|value| async move {
    Ok(Some(value)) // Pass through unchanged
  });

  // Send function
  let _ = function_tx
    .send(Arc::new(FilterMapConfigWrapper::new(passthrough_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: 10, 20, 30
  let test_values = vec![10i32, 20i32, 30i32];

  for value in test_values.clone() {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream
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

  // Should have received all items unchanged
  assert_eq!(results.len(), 3);

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
async fn test_filter_map_error_handling() {
  let node = FilterMapNode::new("test_filter_map".to_string());

  let (_config_tx, in_tx, function_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Create a function that always returns an error
  let error_function: FilterMapConfig =
    filter_map_config(|_value| async move { Err("Test error".to_string()) });

  // Send function
  let _ = function_tx
    .send(Arc::new(FilterMapConfigWrapper::new(error_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: 1, 2
  let test_values = vec![1i32, 2i32];

  for value in test_values {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }
  drop(in_tx); // Close the input stream
  drop(function_tx); // Close the function stream

  let mut out_stream = outputs.remove("out").unwrap();
  let mut error_stream = outputs.remove("error").unwrap();
  let mut output_results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut error_results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();

  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = out_stream.next() => {
        if let Some(item) = result {
          output_results.push(item);
        }
      }
      result = error_stream.next() => {
        if let Some(item) = result {
          error_results.push(item);
        }
      }
      _ = &mut timeout => break,
    }

    if output_results.len() + error_results.len() >= 2 {
      break;
    }
  }

  // Should have received no output items and 2 errors
  assert_eq!(output_results.len(), 0);
  assert_eq!(error_results.len(), 2);

  // Check error messages
  for error_result in error_results {
    if let Ok(error_msg) = error_result.downcast::<String>() {
      assert_eq!(*error_msg, "Test error");
    } else {
      panic!("Error result is not a String");
    }
  }
}
