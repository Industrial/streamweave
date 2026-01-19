//! Tests for TimestampNode

use crate::node::{InputStreams, Node, OutputStreams};
use crate::nodes::time::*;
use futures::StreamExt;
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
async fn test_timestamp_node_creation() {
  let node = TimestampNode::new("test_timestamp".to_string());
  assert_eq!(node.name(), "test_timestamp");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_timestamp_wraps_primitive() {
  let node = TimestampNode::new("test_timestamp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send a primitive value
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
  let first_result = &results[0];
  let first_result_clone = first_result.clone();
  let first_result_double_clone = first_result_clone.clone();
  if let Ok(wrapped_arc) =
    first_result_double_clone.downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    let wrapped: &HashMap<String, Arc<dyn Any + Send + Sync>> = &wrapped_arc;
    // Should have "value" and "timestamp" properties
    assert!(wrapped.contains_key("value"));
    assert!(wrapped.contains_key("timestamp"));

    // Check value
    if let Some(value_arc) = wrapped.get("value") {
      if let Ok(value) = (*value_arc).clone().downcast::<i32>() {
        assert_eq!(*value, 42i32);
      } else {
        panic!("Value is not an i32");
      }
    } else {
      panic!("Missing 'value' property");
    }

    // Check timestamp
    if let Some(timestamp_arc) = wrapped.get("timestamp") {
      if let Ok(timestamp) = (*timestamp_arc).clone().downcast::<i64>() {
        assert!(*timestamp > 0); // Should be a valid timestamp
      } else {
        panic!("Timestamp is not an i64");
      }
    } else {
      panic!("Missing 'timestamp' property");
    }
  } else {
    panic!("Result is not a HashMap");
  }
}

#[tokio::test]
async fn test_timestamp_adds_to_object() {
  let node = TimestampNode::new("test_timestamp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send an object
  let mut obj = HashMap::new();
  obj.insert(
    "name".to_string(),
    Arc::new("test".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  obj.insert(
    "count".to_string(),
    Arc::new(10i32) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in_tx
    .send(Arc::new(obj) as Arc<dyn Any + Send + Sync>)
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
  if let Ok(timestamped_obj) = results[0]
    .clone()
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    // Should have original properties plus timestamp
    assert!(timestamped_obj.contains_key("name"));
    assert!(timestamped_obj.contains_key("count"));
    assert!(timestamped_obj.contains_key("timestamp"));

    // Check original properties are preserved
    if let Some(name_arc) = timestamped_obj.get("name") {
      if let Ok(name) = (*name_arc).clone().downcast::<String>() {
        assert_eq!(*name, "test");
      } else {
        panic!("Name is not a String");
      }
    }

    // Check timestamp
    if let Some(timestamp_arc) = timestamped_obj.get("timestamp") {
      if let Ok(timestamp) = (*timestamp_arc).clone().downcast::<i64>() {
        assert!(*timestamp > 0);
      } else {
        panic!("Timestamp is not an i64");
      }
    }
  } else {
    panic!("Result is not a HashMap");
  }
}

#[tokio::test]
async fn test_timestamp_multiple_items() {
  let node = TimestampNode::new("test_timestamp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send multiple items
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

  assert_eq!(results.len(), 3);

  // Verify each item has a timestamp and timestamps are monotonically increasing
  let mut prev_timestamp: Option<i64> = None;
  for result in results {
    let result_clone = result.clone();
    let result_double_clone = result_clone.clone();
    if let Ok(wrapped_arc) =
      result_double_clone.downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
    {
      let wrapped: &HashMap<String, Arc<dyn Any + Send + Sync>> = &wrapped_arc;
      assert!(wrapped.contains_key("value"));
      assert!(wrapped.contains_key("timestamp"));

      // Verify timestamp is monotonically increasing
      if let Some(timestamp_arc) = wrapped.get("timestamp") {
        if let Ok(timestamp) = (*timestamp_arc).clone().downcast::<i64>() {
          if let Some(prev) = prev_timestamp {
            assert!(
              *timestamp >= prev,
              "Timestamps should be monotonically increasing"
            );
          }
          prev_timestamp = Some(*timestamp);
        }
      }
    } else {
      panic!("Result is not a HashMap");
    }
  }
}

#[tokio::test]
async fn test_timestamp_timing_accuracy() {
  let node = TimestampNode::new("test_timestamp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Record time before sending
  let before_time = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as i64;

  // Send an item
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

  // Record time after receiving
  let after_time = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as i64;

  assert_eq!(results.len(), 1);
  let result = &results[0];
  let cloned_result = result.clone();
  let double_cloned = cloned_result.clone();
  if let Ok(hashmap_arc) = double_cloned.downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>() {
    let hashmap: &HashMap<String, Arc<dyn Any + Send + Sync>> = &hashmap_arc;
    if let Some(timestamp_arc) = hashmap.get("timestamp") {
      let timestamp_arc_clone = (*timestamp_arc).clone();
      if let Ok(timestamp) = timestamp_arc_clone.downcast::<i64>() {
        // Timestamp should be between before_time and after_time (with some tolerance)
        assert!(
          *timestamp >= before_time,
          "Timestamp should be >= before time"
        );
        assert!(
          *timestamp <= after_time + 100,
          "Timestamp should be <= after time (with tolerance)"
        );
      }
    }
  }
}
