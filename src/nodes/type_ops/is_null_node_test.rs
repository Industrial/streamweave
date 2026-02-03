//! Tests for IsNullNode

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::type_ops::IsNullNode;
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
async fn test_is_null_node_creation() {
  let node = IsNullNode::new("test_is_null".to_string());
  assert_eq!(node.name(), "test_is_null");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_is_null_none() {
  let node = IsNullNode::new("test_is_null".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send None (null)
  let _ = in_tx
    .send(Arc::new(None::<()>) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  tokio::select! {


    result = stream.next() => {


      if let Some(item) = result {


        results.push(item);


      }


    }


    _ = &mut timeout => {},


  }

  assert_eq!(results.len(), 1);
  if let Ok(is_nul) = results[0].clone().downcast::<bool>() {
    assert!(*is_nul);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_null_some() {
  let node = IsNullNode::new("test_is_null".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send Some(()) (not null)
  let _ = in_tx
    .send(Arc::new(Some(())) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  tokio::select! {


    result = stream.next() => {


      if let Some(item) = result {


        results.push(item);


      }


    }


    _ = &mut timeout => {},


  }

  assert_eq!(results.len(), 1);
  if let Ok(is_nul) = results[0].clone().downcast::<bool>() {
    assert!(!*is_nul);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_null_i32() {
  let node = IsNullNode::new("test_is_null".to_string());
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

  tokio::select! {


    result = stream.next() => {


      if let Some(item) = result {


        results.push(item);


      }


    }


    _ = &mut timeout => {},


  }

  assert_eq!(results.len(), 1);
  if let Ok(is_nul) = results[0].clone().downcast::<bool>() {
    assert!(!*is_nul);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_null_multiple_types() {
  let node = IsNullNode::new("test_is_null".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send multiple items of different types
  let _ = in_tx
    .send(Arc::new(None::<()>) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
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

  // First item: None (null) -> true
  if let Ok(is_nul) = results[0].clone().downcast::<bool>() {
    assert!(*is_nul);
  } else {
    panic!("First result is not a bool");
  }

  // Second item: i32 (not null) -> false
  if let Ok(is_nul) = results[1].clone().downcast::<bool>() {
    assert!(!*is_nul);
  } else {
    panic!("Second result is not a bool");
  }

  // Third item: String (not null) -> false
  if let Ok(is_nul) = results[2].clone().downcast::<bool>() {
    assert!(!*is_nul);
  } else {
    panic!("Third result is not a bool");
  }
}

#[tokio::test]
async fn test_is_null_stream_completes_properly() {
  // Regression test: Ensure the stream completes when input ends (doesn't hang)
  let node = IsNullNode::new("test_is_null".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send a few items
  let _ = in_tx
    .send(Arc::new(None::<()>) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Close the input channel to signal end of stream
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<bool> = Vec::new();
  let mut stream = out_stream;

  // Use timeout to ensure the stream completes within reasonable time
  let timeout_duration = tokio::time::Duration::from_secs(2);
  let start = std::time::Instant::now();

  loop {
    let timeout_future = tokio::time::sleep(timeout_duration);
    tokio::pin!(timeout_future);

    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
            if results.len() == 3 {
              break; // Got all expected results
            }
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
  assert_eq!(
    results.len(),
    3,
    "Should receive all 3 items before stream ends"
  );
  assert!(results[0]); // None -> true
  assert!(!results[1]); // i32 -> false
  assert!(!results[2]); // String -> false
}
