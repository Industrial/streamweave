#![allow(unused_imports, dead_code, unused, clippy::type_complexity)]

use super::ThrottleNode;
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
  let (period_tx, period_rx) = mpsc::channel(10);

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
    "period".to_string(),
    Box::pin(ReceiverStream::new(period_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, period_tx, inputs)
}

#[tokio::test]
async fn test_throttle_node_creation() {
  let node = ThrottleNode::new("test_throttle".to_string());
  assert_eq!(node.name(), "test_throttle");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("period"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_throttle_node_emits_with_delay() {
  let node = ThrottleNode::new("test_throttle".to_string());

  let (_config_tx, in_tx, period_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Set throttle period: 100ms
  let throttle_period = 100u64;
  period_tx
    .send(Arc::new(throttle_period) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send first item (should be emitted immediately)
  let first_value = 1i32;
  in_tx
    .send(Arc::new(first_value) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Wait less than throttle period and send second item (should be dropped)
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  let second_value = 2i32;
  in_tx
    .send(Arc::new(second_value) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Wait for throttle period to expire and send third item (should be emitted)
  tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;
  let third_value = 3i32;
  in_tx
    .send(Arc::new(third_value) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Close input streams
  drop(in_tx);
  drop(period_tx);

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

  // Should have received only the first and third items (second was throttled)
  assert_eq!(results.len(), 2);

  // Check the values
  if let Ok(value) = results[0].clone().downcast::<i32>() {
    assert_eq!(*value, first_value);
  } else {
    panic!("First result is not an i32");
  }

  if let Ok(value) = results[1].clone().downcast::<i32>() {
    assert_eq!(*value, third_value);
  } else {
    panic!("Third result is not an i32");
  }
}

#[tokio::test]
async fn test_throttle_node_allows_first_item_immediately() {
  let node = ThrottleNode::new("test_throttle".to_string());

  let (_config_tx, in_tx, period_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Set throttle period: 200ms
  let throttle_period = 200u64;
  period_tx
    .send(Arc::new(throttle_period) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send first item (should be emitted immediately since it's the first)
  let first_value = 42i32;
  in_tx
    .send(Arc::new(first_value) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Close input streams
  drop(in_tx);
  drop(period_tx);

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

  // Should have received the first item immediately
  assert_eq!(results.len(), 1);
  if let Ok(value) = results[0].clone().downcast::<i32>() {
    assert_eq!(*value, first_value);
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
async fn test_throttle_node_empty_stream() {
  let node = ThrottleNode::new("test_throttle".to_string());

  let (_config_tx, in_tx, period_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Set throttle period but send no items
  let throttle_period = 50u64;
  period_tx
    .send(Arc::new(throttle_period) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Close input streams
  drop(in_tx);
  drop(period_tx);

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

  // Should have received no items
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_throttle_node_error_handling() {
  let node = ThrottleNode::new("test_throttle".to_string());

  let (_config_tx, in_tx, period_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send invalid period (negative number)
  let invalid_period = -100i32;
  period_tx
    .send(Arc::new(invalid_period) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Close input streams
  drop(in_tx);
  drop(period_tx);

  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(100));
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

  // Should have received an error about negative period
  assert_eq!(errors.len(), 1);
  if let Ok(error_msg) = errors[0].clone().downcast::<String>() {
    assert!(error_msg.contains("cannot be negative"));
  } else {
    panic!("Error result is not a String");
  }
}
