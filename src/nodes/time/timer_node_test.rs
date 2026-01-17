//! Tests for TimerNode

use crate::node::InputStreams;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Helper to create input streams from channels
fn create_input_streams() -> (
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (interval_tx, interval_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "interval".to_string(),
    Box::pin(ReceiverStream::new(interval_rx)) as crate::node::InputStream,
  );

  (config_tx, interval_tx, inputs)
}

#[tokio::test]
async fn test_timer_node_creation() {
  let node = TimerNode::new("test_timer".to_string());
  assert_eq!(node.name(), "test_timer");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("interval"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_timer_generates_events() {
  let node = TimerNode::new("test_timer".to_string());

  let (_config_tx, interval_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send interval: 50 milliseconds
  let _ = interval_tx
    .send(Arc::new(50i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(interval_tx);

  // Wait a bit for timer to start and generate events
  tokio::time::sleep(Duration::from_millis(150)).await;

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
          // Collect a few events
          if results.len() >= 3 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have generated at least 2-3 events in 150ms with 50ms interval
  assert!(results.len() >= 2);

  // Verify events are timestamps (i64)
  for result in results {
    if let Ok(timestamp) = result.downcast::<i64>() {
      assert!(*timestamp > 0); // Should be a valid timestamp
    } else {
      panic!("Result is not an i64 timestamp");
    }
  }
}

#[tokio::test]
async fn test_timer_interval_updates() {
  let node = TimerNode::new("test_timer".to_string());

  let (_config_tx, interval_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send interval: 30 milliseconds
  let _ = interval_tx
    .send(Arc::new(30i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(interval_tx);

  // Wait for timer to generate events
  tokio::time::sleep(Duration::from_millis(100)).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(50));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
          if results.len() >= 2 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have generated at least 2 events
  assert!(results.len() >= 2);
}

#[tokio::test]
async fn test_timer_duration_type() {
  let node = TimerNode::new("test_timer".to_string());

  let (_config_tx, interval_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send interval: 40 milliseconds as Duration
  let _ = interval_tx
    .send(Arc::new(Duration::from_millis(40)) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(interval_tx);

  // Wait for timer to generate events
  tokio::time::sleep(Duration::from_millis(120)).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(50));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
          if results.len() >= 2 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have generated at least 2 events
  assert!(results.len() >= 2);
}

#[tokio::test]
async fn test_timer_invalid_interval() {
  let node = TimerNode::new("test_timer".to_string());

  let (_config_tx, interval_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid interval: negative value
  let _ = interval_tx
    .send(Arc::new(-10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(interval_tx);

  // Check error output
  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<String> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            errors.push((*arc_str).clone());
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("negative"));
}
