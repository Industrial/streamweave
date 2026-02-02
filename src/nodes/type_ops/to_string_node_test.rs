//! Tests for ToStringNode

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::type_ops::ToStringNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams() -> (TestSender, TestSender, InputStreams) {
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
async fn test_to_string_node_creation() {
  let node = ToStringNode::new("test_to_string".to_string());
  assert_eq!(node.name(), "test_to_string");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_to_string_string() {
  let node = ToStringNode::new("test_to_string".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send a String
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

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
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(str_val) = results[0].clone().downcast::<String>() {
    assert_eq!(*str_val, "hello");
  } else {
    panic!("Result is not a String");
  }
}

#[tokio::test]
async fn test_to_string_i32() {
  let node = ToStringNode::new("test_to_string".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send an i32
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

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
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(str_val) = results[0].clone().downcast::<String>() {
    assert_eq!(*str_val, "42");
  } else {
    panic!("Result is not a String");
  }
}

#[tokio::test]
async fn test_to_string_multiple_types() {
  let node = ToStringNode::new("test_to_string".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send multiple items of different types
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
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

  // First item: i32 -> "42"
  if let Ok(str_val) = results[0].clone().downcast::<String>() {
    assert_eq!(*str_val, "42");
  } else {
    panic!("First result is not a String");
  }

  // Second item: String -> "hello"
  if let Ok(str_val) = results[1].clone().downcast::<String>() {
    assert_eq!(*str_val, "hello");
  } else {
    panic!("Second result is not a String");
  }

  // Third item: bool -> "true"
  if let Ok(str_val) = results[2].clone().downcast::<String>() {
    assert_eq!(*str_val, "true");
  } else {
    panic!("Third result is not a String");
  }
}

#[tokio::test]
async fn test_to_string_stream_completes_properly() {
  // Regression test: Ensure the stream completes when input ends (doesn't hang)
  let node = ToStringNode::new("test_to_string".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send a few items
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  
  // Close the input channel to signal end of stream
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  
  // Use timeout to ensure the stream completes within reasonable time
  // This test will fail if the stream hangs (regression test)
  let timeout_duration = tokio::time::Duration::from_secs(2);
  let start = std::time::Instant::now();
  
  loop {
    let timeout_future = tokio::time::sleep(timeout_duration);
    tokio::pin!(timeout_future);
    
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
          if results.len() == 3 {
            break; // Got all expected results
          }
        } else {
          break; // Stream ended (this is expected and good)
        }
      }
      _ = timeout_future => {
        panic!("Stream did not complete within timeout - this indicates a hang bug!");
      }
    }
    
    // Safety check: if we've been running for too long, fail
    if start.elapsed() > timeout_duration {
      panic!("Test took too long - stream may be hanging!");
    }
  }

  // Verify we got all results before the stream ended
  assert_eq!(results.len(), 3, "Should receive all 3 items before stream ends");
  
  // Verify the results
  if let Ok(str_val) = results[0].clone().downcast::<String>() {
    assert_eq!(*str_val, "1");
  } else {
    panic!("First result is not a String");
  }
  if let Ok(str_val) = results[1].clone().downcast::<String>() {
    assert_eq!(*str_val, "2");
  } else {
    panic!("Second result is not a String");
  }
  if let Ok(str_val) = results[2].clone().downcast::<String>() {
    assert_eq!(*str_val, "3");
  } else {
    panic!("Third result is not a String");
  }
}

#[tokio::test]
async fn test_to_string_stream_completes_after_input_closed() {
  // Regression test: Ensure the output stream completes when input stream ends
  // This test verifies that the node doesn't hang waiting for more input
  let node = ToStringNode::new("test_to_string".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send a few items
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  
  // Close the input channel - this should cause the stream to end
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<String> = Vec::new();
  let mut stream = out_stream;
  
  // Collect all results - the stream should end after processing all items
  // Use a reasonable timeout to detect hangs
  let start = std::time::Instant::now();
  let timeout_duration = tokio::time::Duration::from_millis(1000);
  
  loop {
    let timeout = tokio::time::sleep(timeout_duration);
    tokio::pin!(timeout);
    
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(str_val) = item.downcast::<String>() {
            results.push((*str_val).clone());
          }
        } else {
          // Stream ended - this is what we want
          break;
        }
      }
      _ = &mut timeout => {
        // If we timeout, the stream didn't end properly (hang detected)
        panic!("Stream did not complete within timeout - possible hang! Elapsed: {:?}, Results: {:?}", start.elapsed(), results);
      }
    }
    
    // Safety check: if we've been running too long, something is wrong
    if start.elapsed() > timeout_duration {
      panic!("Stream processing took too long - possible hang! Results: {:?}", results);
    }
  }

  // Verify we got all expected results
  assert_eq!(results.len(), 3);
  assert_eq!(results[0], "1");
  assert_eq!(results[1], "2");
  assert_eq!(results[2], "3");
  
  // Verify the stream ended quickly (not hanging)
  assert!(
    start.elapsed() < timeout_duration,
    "Stream should complete quickly after input closes"
  );
}
