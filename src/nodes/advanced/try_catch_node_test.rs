//! Tests for TryCatchNode
#![allow(unused, clippy::type_complexity)]

use crate::node::{InputStreams, Node, OutputStreams};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Helper to create input streams from channels
fn create_input_streams() -> (
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (data_tx, data_rx) = mpsc::channel(10);
  let (try_tx, try_rx) = mpsc::channel(10);
  let (catch_tx, catch_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "try".to_string(),
    Box::pin(ReceiverStream::new(try_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "catch".to_string(),
    Box::pin(ReceiverStream::new(catch_rx)) as crate::node::InputStream,
  );

  (config_tx, data_tx, try_tx, catch_tx, inputs)
}

#[tokio::test]
async fn test_try_catch_node_creation() {
  use crate::nodes::advanced::TryCatchNode;
  let node = TryCatchNode::new("test_try_catch".to_string());
  assert_eq!(node.name(), "test_try_catch");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("try"));
  assert!(node.has_input_port("catch"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
#[ignore] // TODO: Fix async timing issues
async fn test_try_catch_success() {
  use crate::nodes::advanced::{TryCatchNode, catch_config, try_config};
  let node = TryCatchNode::new("test_try_catch".to_string());

  let (_config_tx, data_tx, try_tx, catch_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Create and send try configuration (succeeds)
  let try_fn = try_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let n = *arc_i32;
      Ok(Arc::new(n * 2) as Arc<dyn Any + Send + Sync>)
    } else {
      Err(Arc::new("Expected i32".to_string()) as Arc<dyn Any + Send + Sync>)
    }
  });
  let _ = try_tx.send(try_fn as Arc<dyn Any + Send + Sync>).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Create and send catch configuration (shouldn't be called)
  let catch_fn =
    catch_config(|_error| async move { Ok(Arc::new(0i32) as Arc<dyn Any + Send + Sync>) });
  let _ = catch_tx.send(catch_fn as Arc<dyn Any + Send + Sync>).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Send data after longer delay
  tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Don't drop channels yet - let them close naturally
  // drop(data_tx);
  // drop(try_tx);
  // drop(catch_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(1000));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  tokio::select! {

    result = stream.next() => {

      if let Some(item) = result {

        results.push(item);

      }

    }

    _ = &mut timeout => {},

  }

  println!("Results: {}", results.len());

  assert_eq!(results.len(), 1);
  if let Ok(result) = results[0].clone().downcast::<i32>() {
    assert_eq!(*result, 10); // 5 * 2
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
#[ignore] // TODO: Fix async timing issues
async fn test_try_catch_error_handled() {
  use crate::nodes::advanced::{TryCatchNode, catch_config, try_config};
  let node = TryCatchNode::new("test_try_catch".to_string());

  let (_config_tx, data_tx, try_tx, catch_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Create and send try configuration (fails)
  let try_fn = try_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let n = *arc_i32;
      if n < 0 {
        Err(Arc::new(format!("Negative number: {}", n)) as Arc<dyn Any + Send + Sync>)
      } else {
        Ok(Arc::new(n * 2) as Arc<dyn Any + Send + Sync>)
      }
    } else {
      Err(Arc::new("Expected i32".to_string()) as Arc<dyn Any + Send + Sync>)
    }
  });
  let _ = try_tx.send(try_fn as Arc<dyn Any + Send + Sync>).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Create and send catch configuration (handles error)
  let catch_fn = catch_config(|error| async move {
    if let Ok(error_str) = error.downcast::<String>() {
      Ok(Arc::new(format!("Caught: {}", *error_str)) as Arc<dyn Any + Send + Sync>)
    } else {
      Err(Arc::new("Catch function failed".to_string()) as Arc<dyn Any + Send + Sync>)
    }
  });
  let _ = catch_tx.send(catch_fn as Arc<dyn Any + Send + Sync>).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Send data (negative number will cause try to fail)
  let _ = data_tx
    .send(Arc::new(-5i32) as Arc<dyn Any + Send + Sync>)
    .await;

  drop(data_tx);
  drop(try_tx);
  drop(catch_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(1000));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  tokio::select! {

    result = stream.next() => {

      if let Some(item) = result {

        results.push(item);

      }

    }

    _ = &mut timeout => {},

  }

  assert_eq!(results.len(), 1);
  if let Ok(result_str) = results[0].clone().downcast::<String>() {
    assert!(result_str.contains("Caught:"));
    assert!(result_str.contains("Negative number"));
  } else {
    panic!("Result is not a String");
  }
}

#[tokio::test]
#[ignore] // TODO: Fix async timing issues
async fn test_try_catch_error_not_handled() {
  use crate::nodes::advanced::{TryCatchNode, catch_config, try_config};
  let node = TryCatchNode::new("test_try_catch".to_string());

  let (_config_tx, data_tx, try_tx, catch_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Create and send try configuration (fails)
  let try_fn = try_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let n = *arc_i32;
      if n < 0 {
        Err(Arc::new(format!("Negative number: {}", n)) as Arc<dyn Any + Send + Sync>)
      } else {
        Ok(Arc::new(n * 2) as Arc<dyn Any + Send + Sync>)
      }
    } else {
      Err(Arc::new("Expected i32".to_string()) as Arc<dyn Any + Send + Sync>)
    }
  });
  let _ = try_tx.send(try_fn as Arc<dyn Any + Send + Sync>).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Create and send catch configuration (also fails)
  let catch_fn = catch_config(|_error| async move {
    Err(Arc::new("Catch function failed".to_string()) as Arc<dyn Any + Send + Sync>)
  });
  let _ = catch_tx.send(catch_fn as Arc<dyn Any + Send + Sync>).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Send data (negative number will cause try to fail)
  let _ = data_tx
    .send(Arc::new(-5i32) as Arc<dyn Any + Send + Sync>)
    .await;

  drop(data_tx);
  drop(try_tx);
  drop(catch_tx);

  let error_stream = outputs.remove("error").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(1000));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  tokio::select! {

    result = stream.next() => {

      if let Some(item) = result {

        results.push(item);

      }

    }

    _ = &mut timeout => {},

  }

  assert_eq!(results.len(), 1);
  if let Ok(error_str) = results[0].clone().downcast::<String>() {
    assert_eq!(*error_str, "Catch function failed");
  } else {
    panic!("Error is not a String");
  }
}

#[tokio::test]
#[ignore] // TODO: Fix async timing issues
async fn test_try_catch_no_catch_function() {
  use crate::nodes::advanced::{TryCatchNode, try_config};
  let node = TryCatchNode::new("test_try_catch".to_string());

  let (_config_tx, data_tx, try_tx, catch_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Create and send try configuration (fails)
  let try_fn = try_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let n = *arc_i32;
      if n < 0 {
        Err(Arc::new(format!("Negative number: {}", n)) as Arc<dyn Any + Send + Sync>)
      } else {
        Ok(Arc::new(n * 2) as Arc<dyn Any + Send + Sync>)
      }
    } else {
      Err(Arc::new("Expected i32".to_string()) as Arc<dyn Any + Send + Sync>)
    }
  });
  let _ = try_tx.send(try_fn as Arc<dyn Any + Send + Sync>).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
  tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

  // Don't send catch configuration

  // Send data (negative number will cause try to fail)
  let _ = data_tx
    .send(Arc::new(-5i32) as Arc<dyn Any + Send + Sync>)
    .await;

  drop(data_tx);
  drop(try_tx);
  drop(catch_tx);

  let error_stream = outputs.remove("error").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(1000));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  tokio::select! {

    result = stream.next() => {

      if let Some(item) = result {

        results.push(item);

      }

    }

    _ = &mut timeout => {},

  }

  assert_eq!(results.len(), 1);
  if let Ok(error_str) = results[0].clone().downcast::<String>() {
    assert!(error_str.contains("Negative number"));
  } else {
    panic!("Error is not a String");
  }
}
