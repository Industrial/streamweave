//! Tests for JoinNode

use crate::node::{InputStreams, Node};
use crate::nodes::join_node::{JoinNode, JoinStrategy, join_config};
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
  let (left_tx, left_rx) = mpsc::channel(10);
  let (right_tx, right_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "left".to_string(),
    Box::pin(ReceiverStream::new(left_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "right".to_string(),
    Box::pin(ReceiverStream::new(right_rx)) as crate::node::InputStream,
  );

  (config_tx, left_tx, right_tx, inputs)
}

#[tokio::test]
async fn test_join_node_creation() {
  let node = JoinNode::new("test_join".to_string());
  assert_eq!(node.name(), "test_join");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("left"));
  assert!(node.has_input_port("right"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
  assert!(!node.has_config());
}

#[tokio::test]
async fn test_join_node_inner_join() {
  let node = JoinNode::new("test_join".to_string());

  let (config_tx, left_tx, right_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Helper function for key extraction
  let key_fn = |item: Arc<dyn Any + Send + Sync>| {
    Box::pin(async move {
      if let Ok(arc_map) = item.downcast::<HashMap<String, i32>>() {
        if let Some(id) = arc_map.get("id") {
          Ok(id.to_string())
        } else {
          Err("Missing 'id' key".to_string())
        }
      } else {
        Err("Expected HashMap<String, i32>".to_string())
      }
    })
  };

  // Create configuration for inner join
  // Key extraction: extract "id" field from TestRecord
  // For simplicity, we'll use HashMap<String, i32> as test data
  let config = join_config(
    JoinStrategy::Inner,
    key_fn,
    key_fn,
    |left: Arc<dyn Any + Send + Sync>, right: Option<Arc<dyn Any + Send + Sync>>| {
      Box::pin(async move {
        // Combine left and right into a tuple-like structure
        // For simplicity, return a HashMap with both
        let mut result = HashMap::new();
        result.insert("left".to_string(), left);
        if let Some(r) = right {
          result.insert("right".to_string(), r);
        }
        Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
      })
    },
  );

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Send left item
  let mut left_item = HashMap::new();
  left_item.insert("id".to_string(), 1);
  left_item.insert("name".to_string(), 10); // Using i32 for simplicity
  let _ = left_tx
    .send(Arc::new(left_item) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send right item with matching key
  let mut right_item = HashMap::new();
  right_item.insert("id".to_string(), 1);
  right_item.insert("value".to_string(), 100);
  let _ = right_tx
    .send(Arc::new(right_item) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

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
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  // Verify the joined result contains both left and right
  if let Ok(arc_result) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    assert!(arc_result.contains_key("left"));
    assert!(arc_result.contains_key("right"));
  } else {
    panic!("Expected HashMap result");
  }
}

#[tokio::test]
async fn test_join_node_left_join() {
  let node = JoinNode::new("test_join".to_string());

  let (config_tx, left_tx, _right_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Helper function for key extraction
  let key_fn = |item: Arc<dyn Any + Send + Sync>| {
    Box::pin(async move {
      if let Ok(arc_map) = item.downcast::<HashMap<String, i32>>() {
        if let Some(id) = arc_map.get("id") {
          Ok(id.to_string())
        } else {
          Err("Missing 'id' key".to_string())
        }
      } else {
        Err("Expected HashMap<String, i32>".to_string())
      }
    })
  };

  // Create configuration for left join
  let config = join_config(
    JoinStrategy::Left,
    key_fn,
    key_fn,
    |left: Arc<dyn Any + Send + Sync>, right: Option<Arc<dyn Any + Send + Sync>>| {
      Box::pin(async move {
        let mut result = HashMap::new();
        result.insert("left".to_string(), left);
        if let Some(r) = right {
          result.insert("right".to_string(), r);
        }
        Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
      })
    },
  );

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Send left item with no matching right item
  let mut left_item = HashMap::new();
  left_item.insert("id".to_string(), 1);
  left_item.insert("name".to_string(), 10);
  let _ = left_tx
    .send(Arc::new(left_item) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

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
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Left join should emit even without a match
  assert_eq!(results.len(), 1);
  if let Ok(arc_result) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    assert!(arc_result.contains_key("left"));
    // Right should be None (not present in result)
    assert!(!arc_result.contains_key("right"));
  } else {
    panic!("Expected HashMap result");
  }
}

#[tokio::test]
async fn test_join_node_error_no_config() {
  let node = JoinNode::new("test_join".to_string());

  let (_config_tx, left_tx, _right_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send data without configuration
  let mut left_item = HashMap::new();
  left_item.insert("id".to_string(), 1);
  let _ = left_tx
    .send(Arc::new(left_item) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect error
  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_string) = item.downcast::<String>() {
            errors.push(arc_string.clone());
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("No configuration set"));
}

#[tokio::test]
async fn test_join_node_right_join() {
  let node = JoinNode::new("test_join".to_string());

  let (config_tx, _left_tx, right_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Helper function for key extraction
  let key_fn = |item: Arc<dyn Any + Send + Sync>| {
    Box::pin(async move {
      if let Ok(arc_map) = item.downcast::<HashMap<String, i32>>() {
        if let Some(id) = arc_map.get("id") {
          Ok(id.to_string())
        } else {
          Err("Missing 'id' key".to_string())
        }
      } else {
        Err("Expected HashMap<String, i32>".to_string())
      }
    })
  };

  // Create configuration for right join
  let config = join_config(
    JoinStrategy::Right,
    key_fn,
    key_fn,
    |left: Arc<dyn Any + Send + Sync>, right: Option<Arc<dyn Any + Send + Sync>>| {
      Box::pin(async move {
        let mut result = HashMap::new();
        if let Ok(l) = left.downcast::<HashMap<String, i32>>() {
          result.insert(
            "left".to_string(),
            Arc::new(l.clone()) as Arc<dyn Any + Send + Sync>,
          );
        }
        if let Some(r) = right {
          result.insert("right".to_string(), r);
        }
        Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
      })
    },
  );

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Send right item with no matching left item
  let mut right_item = HashMap::new();
  right_item.insert("id".to_string(), 2);
  right_item.insert("value".to_string(), 200);
  let _ = right_tx
    .send(Arc::new(right_item) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

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
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Right join should emit even without a match
  assert_eq!(results.len(), 1);
}
