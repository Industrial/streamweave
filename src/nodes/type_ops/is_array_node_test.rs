//! Tests for IsArrayNode

use crate::node::InputStreams;
use crate::node::Node;
use crate::nodes::common::TestSender;
use crate::nodes::type_ops::IsArrayNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams() -> (TestSender, TestSender, InputStreams) {
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
async fn test_is_array_node_creation() {
  let node = IsArrayNode::new("test_is_array".to_string());
  assert_eq!(node.name(), "test_is_array");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_is_array_array() {
  let node = IsArrayNode::new("test_is_array".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send an array
  let array: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(array) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
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
  if let Ok(is_arr) = results[0].clone().downcast::<bool>() {
    assert!(*is_arr);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_array_i32() {
  let node = IsArrayNode::new("test_is_array".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send an i32
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
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
  if let Ok(is_arr) = results[0].clone().downcast::<bool>() {
    assert!(!*is_arr);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_array_string() {
  let node = IsArrayNode::new("test_is_array".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send a String
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
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
  if let Ok(is_arr) = results[0].clone().downcast::<bool>() {
    assert!(!*is_arr);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_array_bool() {
  let node = IsArrayNode::new("test_is_array".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send a bool
  let _ = in_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
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
  if let Ok(is_arr) = results[0].clone().downcast::<bool>() {
    assert!(!*is_arr);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_array_object() {
  let node = IsArrayNode::new("test_is_array".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send an object
  let mut obj = HashMap::new();
  obj.insert(
    "key".to_string(),
    Arc::new("value".to_string()) as Arc<dyn Any + Send + Sync>,
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

  tokio::select! {


    result = stream.next() => {


      if let Some(item) = result {


        results.push(item);


      }


    }


    _ = &mut timeout => {},


  }

  assert_eq!(results.len(), 1);
  if let Ok(is_arr) = results[0].clone().downcast::<bool>() {
    assert!(!*is_arr);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_array_empty_array() {
  let node = IsArrayNode::new("test_is_array".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send an empty array
  let array: Vec<Arc<dyn Any + Send + Sync>> = vec![];
  let _ = in_tx
    .send(Arc::new(array) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
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
  if let Ok(is_arr) = results[0].clone().downcast::<bool>() {
    assert!(*is_arr); // Empty array is still an array
  } else {
    panic!("Result is not a bool");
  }
}
