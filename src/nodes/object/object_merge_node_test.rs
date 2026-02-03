//! Tests for ObjectMergeNode

use crate::node::{InputStreams, Node};
use crate::nodes::object::object_merge_node::ObjectMergeNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

type AnySender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels
fn create_input_streams() -> (AnySender, AnySender, AnySender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in1_tx, in1_rx) = mpsc::channel(10);
  let (in2_tx, in2_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in1".to_string(),
    Box::pin(ReceiverStream::new(in1_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in2".to_string(),
    Box::pin(ReceiverStream::new(in2_rx)) as crate::node::InputStream,
  );

  (config_tx, in1_tx, in2_tx, inputs)
}

#[tokio::test]
async fn test_object_merge_node_creation() {
  let node = ObjectMergeNode::new("test_merge".to_string());
  assert_eq!(node.name(), "test_merge");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in1"));
  assert!(node.has_input_port("in2"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_object_merge_basic() {
  let node = ObjectMergeNode::new("test_merge".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: {"name": "John"} + {"age": 30} → {"name": "John", "age": 30}
  let mut map1: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  map1.insert(
    "name".to_string(),
    Arc::new("John".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  let mut map2: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  map2.insert(
    "age".to_string(),
    Arc::new(30i32) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in1_tx
    .send(Arc::new(map1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(map2) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
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
  if let Ok(merged_map) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(merged_map.len(), 2);
    let name_value = merged_map
      .get("name")
      .unwrap()
      .clone()
      .downcast::<String>()
      .unwrap();
    let age_value = merged_map
      .get("age")
      .unwrap()
      .clone()
      .downcast::<i32>()
      .unwrap();
    assert_eq!(*name_value, "John".to_string());
    assert_eq!(*age_value, 30i32);
  } else {
    panic!("Result is not a HashMap");
  }
}

#[tokio::test]
async fn test_object_merge_key_conflict() {
  let node = ObjectMergeNode::new("test_merge".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: {"name": "John"} + {"name": "Jane"} → {"name": "Jane"} (second overwrites first)
  let mut map1: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  map1.insert(
    "name".to_string(),
    Arc::new("John".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  let mut map2: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  map2.insert(
    "name".to_string(),
    Arc::new("Jane".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in1_tx
    .send(Arc::new(map1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(map2) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
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
  if let Ok(merged_map) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(merged_map.len(), 1);
    let name_value = merged_map
      .get("name")
      .unwrap()
      .clone()
      .downcast::<String>()
      .unwrap();
    assert_eq!(*name_value, "Jane".to_string()); // Second object's value overwrites first
  } else {
    panic!("Result is not a HashMap");
  }
}

#[tokio::test]
async fn test_object_merge_empty_first() {
  let node = ObjectMergeNode::new("test_merge".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: {} + {"age": 30} → {"age": 30}
  let map1: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  let mut map2: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  map2.insert(
    "age".to_string(),
    Arc::new(30i32) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in1_tx
    .send(Arc::new(map1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(map2) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
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
  if let Ok(merged_map) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(merged_map.len(), 1);
    let age_value = merged_map
      .get("age")
      .unwrap()
      .clone()
      .downcast::<i32>()
      .unwrap();
    assert_eq!(*age_value, 30i32);
  } else {
    panic!("Result is not a HashMap");
  }
}

#[tokio::test]
async fn test_object_merge_empty_second() {
  let node = ObjectMergeNode::new("test_merge".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: {"name": "John"} + {} → {"name": "John"}
  let mut map1: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  map1.insert(
    "name".to_string(),
    Arc::new("John".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  let map2: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  let _ = in1_tx
    .send(Arc::new(map1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(map2) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
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
  if let Ok(merged_map) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(merged_map.len(), 1);
    let name_value = merged_map
      .get("name")
      .unwrap()
      .clone()
      .downcast::<String>()
      .unwrap();
    assert_eq!(*name_value, "John".to_string());
  } else {
    panic!("Result is not a HashMap");
  }
}

#[tokio::test]
async fn test_object_merge_non_object_input() {
  let node = ObjectMergeNode::new("test_merge".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-object input
  let _ = in1_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let mut map2: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  map2.insert(
    "age".to_string(),
    Arc::new(30i32) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in2_tx
    .send(Arc::new(map2) as Arc<dyn Any + Send + Sync>)
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
