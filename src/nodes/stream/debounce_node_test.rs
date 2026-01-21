#![allow(unused_imports, dead_code, unused, clippy::type_complexity)]

use super::DebounceNode;
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
  let (delay_tx, delay_rx) = mpsc::channel(10);

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
    "delay".to_string(),
    Box::pin(ReceiverStream::new(delay_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, delay_tx, inputs)
}

#[tokio::test]
async fn test_debounce_node_creation() {
  let node = DebounceNode::new("test_debounce".to_string());
  assert_eq!(node.name(), "test_debounce");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("delay"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_debounce_single_item() {
  let node = DebounceNode::new("test_debounce".to_string());

  let (_config_tx, in_tx, delay_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send debounce delay: 50ms
  let debounce_delay = 50u64;
  delay_tx
    .send(Arc::new(debounce_delay) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send single item
  let test_value = 42i32;
  in_tx
    .send(Arc::new(test_value) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Close input streams
  drop(in_tx);
  drop(delay_tx);

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

  // Should have received 1 item after the debounce delay
  assert_eq!(results.len(), 1);
  if let Ok(value) = results[0].clone().downcast::<i32>() {
    assert_eq!(*value, test_value);
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
async fn test_debounce_multiple_items_with_cancellation() {
  let node = DebounceNode::new("test_debounce".to_string());

  let (_config_tx, in_tx, delay_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send debounce delay: 100ms
  let debounce_delay = 100u64;
  delay_tx
    .send(Arc::new(debounce_delay) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send first item
  in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Wait less than debounce delay and send second item (should cancel first)
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Wait less than debounce delay and send third item (should cancel second)
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Close input streams
  drop(in_tx);
  drop(delay_tx);

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

  // Should have received only the final item (3) after the debounce delay
  // The first two items should have been cancelled
  assert_eq!(results.len(), 1);
  if let Ok(value) = results[0].clone().downcast::<i32>() {
    assert_eq!(*value, 3i32);
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
async fn test_debounce_immediate_emission_on_stream_end() {
  let node = DebounceNode::new("test_debounce".to_string());

  let (_config_tx, in_tx, delay_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Send debounce delay: 200ms (long delay)
  let debounce_delay = 200u64;
  delay_tx
    .send(Arc::new(debounce_delay) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send item and immediately close stream (before delay expires)
  let test_value = 99i32;
  in_tx
    .send(Arc::new(test_value) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Close input streams immediately
  drop(in_tx);
  drop(delay_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(100));
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

  // Should have received the item immediately when stream ended
  // (since no more items can cancel the debounce)
  assert_eq!(results.len(), 1);
  if let Ok(value) = results[0].clone().downcast::<i32>() {
    assert_eq!(*value, test_value);
  } else {
    panic!("Result is not an i32");
  }
}
