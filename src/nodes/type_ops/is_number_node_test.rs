//! Tests for IsNumberNode

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::type_ops::IsNumberNode;
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
async fn test_is_number_node_creation() {
  let node = IsNumberNode::new("test_is_number".to_string());
  assert_eq!(node.name(), "test_is_number");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_is_number_i32() {
  let node = IsNumberNode::new("test_is_number".to_string());
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
  if let Ok(is_num) = results[0].clone().downcast::<bool>() {
    assert!(*is_num);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_number_f64() {
  let node = IsNumberNode::new("test_is_number".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send an f64
  let _ = in_tx
    .send(Arc::new(2.5f64) as Arc<dyn Any + Send + Sync>)
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
  if let Ok(is_num) = results[0].clone().downcast::<bool>() {
    assert!(*is_num);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_number_string() {
  let node = IsNumberNode::new("test_is_number".to_string());
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
  if let Ok(is_num) = results[0].clone().downcast::<bool>() {
    assert!(!*is_num);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_number_bool() {
  let node = IsNumberNode::new("test_is_number".to_string());
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
  if let Ok(is_num) = results[0].clone().downcast::<bool>() {
    assert!(!*is_num);
  } else {
    panic!("Result is not a bool");
  }
}

#[tokio::test]
async fn test_is_number_multiple_types() {
  let node = IsNumberNode::new("test_is_number".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send multiple items of different types
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(500));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
          if results.len() == 4 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 4);

  // First item: i32 (number) -> true
  if let Ok(is_num) = results[0].clone().downcast::<bool>() {
    assert!(*is_num);
  } else {
    panic!("First result is not a bool");
  }

  // Second item: String (not number) -> false
  if let Ok(is_num) = results[1].clone().downcast::<bool>() {
    assert!(!*is_num);
  } else {
    panic!("Second result is not a bool");
  }

  // Third item: f64 (number) -> true
  if let Ok(is_num) = results[2].clone().downcast::<bool>() {
    assert!(*is_num);
  } else {
    panic!("Third result is not a bool");
  }

  // Fourth item: bool (not number) -> false
  if let Ok(is_num) = results[3].clone().downcast::<bool>() {
    assert!(!*is_num);
  } else {
    panic!("Fourth result is not a bool");
  }
}
