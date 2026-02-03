//! Tests for ObjectSetPropertyNode

use crate::node::{InputStreams, Node};
use crate::nodes::object::object_set_property_node::ObjectSetPropertyNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

type AnySender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels
fn create_input_streams() -> (AnySender, AnySender, AnySender, AnySender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (key_tx, key_rx) = mpsc::channel(10);
  let (value_tx, value_rx) = mpsc::channel(10);

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
    "key".to_string(),
    Box::pin(ReceiverStream::new(key_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "value".to_string(),
    Box::pin(ReceiverStream::new(value_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, key_tx, value_tx, inputs)
}

#[tokio::test]
async fn test_object_set_property_node_creation() {
  let node = ObjectSetPropertyNode::new("test_set_property".to_string());
  assert_eq!(node.name(), "test_set_property");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("key"));
  assert!(node.has_input_port("value"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_object_set_property_new() {
  let node = ObjectSetPropertyNode::new("test_set_property".to_string());

  let (_config_tx, in_tx, key_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: {"name": "John"} with key "age" and value 30 → {"name": "John", "age": 30}
  let mut map: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  map.insert(
    "name".to_string(),
    Arc::new("John".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in_tx
    .send(Arc::new(map) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("age".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new(30i32) as Arc<dyn Any + Send + Sync>)
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
  if let Ok(new_map) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(new_map.len(), 2);
    let name_value = new_map
      .get("name")
      .unwrap()
      .clone()
      .downcast::<String>()
      .unwrap();
    let age_value = new_map
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
async fn test_object_set_property_overwrite() {
  let node = ObjectSetPropertyNode::new("test_set_property".to_string());

  let (_config_tx, in_tx, key_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: {"name": "John"} with key "name" and value "Jane" → {"name": "Jane"}
  let mut map: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  map.insert(
    "name".to_string(),
    Arc::new("John".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in_tx
    .send(Arc::new(map) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("name".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new("Jane".to_string()) as Arc<dyn Any + Send + Sync>)
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
  if let Ok(new_map) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(new_map.len(), 1);
    let name_value = new_map
      .get("name")
      .unwrap()
      .clone()
      .downcast::<String>()
      .unwrap();
    assert_eq!(*name_value, "Jane".to_string()); // Value was overwritten
  } else {
    panic!("Result is not a HashMap");
  }
}

#[tokio::test]
async fn test_object_set_property_empty_object() {
  let node = ObjectSetPropertyNode::new("test_set_property".to_string());

  let (_config_tx, in_tx, key_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: {} with key "name" and value "John" → {"name": "John"}
  let map: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  let _ = in_tx
    .send(Arc::new(map) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("name".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new("John".to_string()) as Arc<dyn Any + Send + Sync>)
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
  if let Ok(new_map) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(new_map.len(), 1);
    let name_value = new_map
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
async fn test_object_set_property_non_object_input() {
  let node = ObjectSetPropertyNode::new("test_set_property".to_string());

  let (_config_tx, in_tx, key_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-object input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("name".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new("John".to_string()) as Arc<dyn Any + Send + Sync>)
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

#[tokio::test]
async fn test_object_set_property_invalid_key_type() {
  let node = ObjectSetPropertyNode::new("test_set_property".to_string());

  let (_config_tx, in_tx, key_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send object with non-string key
  let mut map: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  map.insert(
    "name".to_string(),
    Arc::new("John".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in_tx
    .send(Arc::new(map) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new("Jane".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("key must be String"));
}
