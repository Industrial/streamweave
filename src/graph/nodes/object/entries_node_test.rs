//! Tests for ObjectEntriesNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::object::ObjectEntriesNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

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
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, inputs)
}

#[tokio::test]
async fn test_object_entries_node_creation() {
  let node = ObjectEntriesNode::new("test_entries".to_string());
  assert_eq!(node.name(), "test_entries");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_object_entries_basic() {
  let node = ObjectEntriesNode::new("test_entries".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: {"name": "John", "age": 30} → [["name", "John"], ["age", 30]]
  let mut map: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  map.insert(
    "name".to_string(),
    Arc::new("John".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  map.insert(
    "age".to_string(),
    Arc::new(30i32) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in_tx
    .send(Arc::new(map) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_vec) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
            results.push(arc_vec.clone());
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
  assert_eq!(results[0].len(), 2); // Two entries

  // Verify entries are present (order may vary)
  let mut has_name_entry = false;
  let mut has_age_entry = false;

  for entry in results[0].iter() {
    if let Ok(arc_pair) = entry.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
      assert_eq!(arc_pair.len(), 2); // Each entry is [key, value]
      let key = arc_pair[0].clone().downcast::<String>().unwrap();
      if *key == "name".to_string() {
        let value = arc_pair[1].clone().downcast::<String>().unwrap();
        assert_eq!(*value, "John".to_string());
        has_name_entry = true;
      } else if *key == "age".to_string() {
        let value = arc_pair[1].clone().downcast::<i32>().unwrap();
        assert_eq!(*value, 30i32);
        has_age_entry = true;
      }
    }
  }

  assert!(has_name_entry);
  assert!(has_age_entry);
}

#[tokio::test]
async fn test_object_entries_empty() {
  let node = ObjectEntriesNode::new("test_entries".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: {} → []
  let map: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  let _ = in_tx
    .send(Arc::new(map) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_vec) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
            results.push(arc_vec.clone());
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
  assert_eq!(results[0].len(), 0);
}

#[tokio::test]
async fn test_object_entries_non_object_input() {
  let node = ObjectEntriesNode::new("test_entries".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-object input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            errors.push(arc_str.clone());
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
  assert!(errors[0].contains("input must be HashMap"));
}
