//! Tests for CurrentTimeNode

use crate::node::{InputStreams, Node};
use crate::nodes::time::CurrentTimeNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
type AnySender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels
fn create_input_streams() -> (AnySender, AnySender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (trigger_tx, trigger_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "trigger".to_string(),
    Box::pin(ReceiverStream::new(trigger_rx)) as crate::node::InputStream,
  );

  (config_tx, trigger_tx, inputs)
}

#[tokio::test]
async fn test_current_time_node_creation() {
  let node = CurrentTimeNode::new("test_current_time".to_string());
  assert_eq!(node.name(), "test_current_time");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("trigger"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_current_time_node_basic() {
  let node = CurrentTimeNode::new("test_current_time".to_string());

  let (_config_tx, trigger_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send a trigger signal
  let _ = trigger_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_timestamp) = item.downcast::<i64>() {
            results.push(*arc_timestamp);
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);

  // Verify timestamp is reasonable (within last second)
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_millis() as i64;
  let timestamp = results[0];

  // Should be within 1 second of current time
  assert!(
    (now - timestamp).abs() < 1000,
    "Timestamp {} should be close to current time {}",
    timestamp,
    now
  );
}

#[tokio::test]
async fn test_current_time_node_multiple_triggers() {
  let node = CurrentTimeNode::new("test_current_time".to_string());

  let (_config_tx, trigger_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send multiple trigger signals
  for _ in 0..3 {
    let _ = trigger_tx
      .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
      .await;
  }

  // Collect results
  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;

  // Collect up to 3 items with timeout
  for _ in 0..3 {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_timestamp) = item.downcast::<i64>() {
            results.push(*arc_timestamp);
          }
        } else {
          break;
        }
      }
      _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
        break;
      }
    }
  }

  assert_eq!(results.len(), 3);

  // Verify all timestamps are reasonable and increasing
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_millis() as i64;

  for &timestamp in &results {
    assert!(
      (now - timestamp).abs() < 1000,
      "Timestamp {} should be close to current time {}",
      timestamp,
      now
    );
  }

  // Timestamps should be monotonically increasing (though they might be very close)
  assert!(
    results[1] >= results[0],
    "Second timestamp should be >= first"
  );
  assert!(
    results[2] >= results[1],
    "Third timestamp should be >= second"
  );
}
