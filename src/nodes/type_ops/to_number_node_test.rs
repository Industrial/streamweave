//! Tests for ToNumberNode

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::type_ops::ToNumberNode;
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
async fn test_to_number_node_creation() {
  let node = ToNumberNode::new("test_to_number".to_string());
  assert_eq!(node.name(), "test_to_number");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_to_number_i32() {
  let node = ToNumberNode::new("test_to_number".to_string());
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
  if let Ok(num_val) = results[0].clone().downcast::<f64>() {
    assert_eq!(*num_val, 42.0);
  } else {
    panic!("Result is not an f64");
  }
}

#[tokio::test]
async fn test_to_number_f64() {
  let node = ToNumberNode::new("test_to_number".to_string());
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
  if let Ok(num_val) = results[0].clone().downcast::<f64>() {
    assert_eq!(*num_val, 2.5);
  } else {
    panic!("Result is not an f64");
  }
}

#[tokio::test]
async fn test_to_number_string_valid() {
  let node = ToNumberNode::new("test_to_number".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send a valid numeric string
  let _ = in_tx
    .send(Arc::new("42.5".to_string()) as Arc<dyn Any + Send + Sync>)
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
  if let Ok(num_val) = results[0].clone().downcast::<f64>() {
    assert_eq!(*num_val, 42.5);
  } else {
    panic!("Result is not an f64");
  }
}

#[tokio::test]
async fn test_to_number_string_invalid() {
  let node = ToNumberNode::new("test_to_number".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send an invalid numeric string
  let _ = in_tx
    .send(Arc::new("not a number".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);

  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  tokio::select! {


    result = stream.next() => {


      if let Some(item) = result {


        errors.push(item);


      }


    }


    _ = &mut timeout => {},


  }

  assert_eq!(errors.len(), 1);
  if let Ok(err_str) = errors[0].clone().downcast::<String>() {
    assert!(err_str.contains("Cannot parse string to number"));
  } else {
    panic!("Error is not a String");
  }
}

#[tokio::test]
async fn test_to_number_multiple_types() {
  let node = ToNumberNode::new("test_to_number".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send multiple items of different types
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new("2.5".to_string()) as Arc<dyn Any + Send + Sync>)
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

  // First item: i32 -> 42.0
  if let Ok(num_val) = results[0].clone().downcast::<f64>() {
    assert_eq!(*num_val, 42.0);
  } else {
    panic!("First result is not an f64");
  }

  // Second item: String "2.5" -> 2.5
  if let Ok(num_val) = results[1].clone().downcast::<f64>() {
    assert_eq!(*num_val, 2.5);
  } else {
    panic!("Second result is not an f64");
  }

  // Third item: bool true -> 1.0
  if let Ok(num_val) = results[2].clone().downcast::<f64>() {
    assert_eq!(*num_val, 1.0);
  } else {
    panic!("Third result is not an f64");
  }
}
