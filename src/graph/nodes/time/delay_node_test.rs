//! Tests for DelayNode

use crate::graph::node::InputStreams;
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
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (duration_tx, duration_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "duration".to_string(),
    Box::pin(ReceiverStream::new(duration_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, duration_tx, inputs)
}

#[tokio::test]
async fn test_delay_node_creation() {
  let node = DelayNode::new("test_delay".to_string());
  assert_eq!(node.name(), "test_delay");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("duration"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_delay_i32_milliseconds() {
  let node = DelayNode::new("test_delay".to_string());

  let (_config_tx, in_tx, duration_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send duration: 50 milliseconds
  let _ = duration_tx
    .send(Arc::new(50i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send item
  let start = Instant::now();
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(duration_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          let elapsed = start.elapsed();
          results.push(item);
          // Verify delay was approximately 50ms (allow some tolerance)
          assert!(elapsed >= Duration::from_millis(45));
          assert!(elapsed <= Duration::from_millis(100));
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(value) = results[0].clone().downcast::<i32>() {
    assert_eq!(*value, 42i32);
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
async fn test_delay_duration_type() {
  let node = DelayNode::new("test_delay".to_string());

  let (_config_tx, in_tx, duration_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send duration: 30 milliseconds as Duration
  let _ = duration_tx
    .send(Arc::new(Duration::from_millis(30)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send item
  let start = Instant::now();
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(duration_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          let elapsed = start.elapsed();
          results.push(item);
          // Verify delay was approximately 30ms
          assert!(elapsed >= Duration::from_millis(25));
          assert!(elapsed <= Duration::from_millis(80));
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(value) = results[0].clone().downcast::<String>() {
    assert_eq!(*value, "hello");
  } else {
    panic!("Result is not a String");
  }
}

#[tokio::test]
async fn test_delay_multiple_items() {
  let node = DelayNode::new("test_delay".to_string());

  let (_config_tx, in_tx, duration_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send duration: 20 milliseconds
  let _ = duration_tx
    .send(Arc::new(20i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send multiple items
  let start = Instant::now();
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(duration_tx);

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

  let elapsed = start.elapsed();
  assert_eq!(results.len(), 3);
  // Each item should be delayed by 20ms, so total should be at least 60ms
  assert!(elapsed >= Duration::from_millis(55));

  // Verify order is preserved
  if let (Ok(val1), Ok(val2), Ok(val3)) = (
    results[0].clone().downcast::<i32>(),
    results[1].clone().downcast::<i32>(),
    results[2].clone().downcast::<i32>(),
  ) {
    assert_eq!(*val1, 1i32);
    assert_eq!(*val2, 2i32);
    assert_eq!(*val3, 3i32);
  } else {
    panic!("Results are not i32");
  }
}

#[tokio::test]
async fn test_delay_invalid_duration() {
  let node = DelayNode::new("test_delay".to_string());

  let (_config_tx, in_tx, duration_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid duration: negative value
  let _ = duration_tx
    .send(Arc::new(-10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(duration_tx);

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

  drop(in_tx);
}
