//! Tests for TypeOfNode

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::type_ops::TypeOfNode;
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
async fn test_type_of_node_creation() {
  let node = TypeOfNode::new("test_type_of".to_string());
  assert_eq!(node.name(), "test_type_of");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_type_of_i32() {
  let node = TypeOfNode::new("test_type_of".to_string());
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
  if let Ok(type_name) = results[0].clone().downcast::<String>() {
    // All data flows as Arc<dyn Any + Send + Sync>, so type_name_of_val returns the trait object type
    assert_eq!(
      *type_name,
      "dyn core::any::Any + core::marker::Send + core::marker::Sync"
    );
  } else {
    panic!("Result is not a String");
  }
}

#[tokio::test]
async fn test_type_of_string() {
  let node = TypeOfNode::new("test_type_of".to_string());
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
  if let Ok(type_name) = results[0].clone().downcast::<String>() {
    // All data flows as Arc<dyn Any + Send + Sync>, so type_name_of_val returns the trait object type
    assert_eq!(
      *type_name,
      "dyn core::any::Any + core::marker::Send + core::marker::Sync"
    );
  } else {
    panic!("Result is not a String");
  }
}

#[tokio::test]
async fn test_type_of_bool() {
  let node = TypeOfNode::new("test_type_of".to_string());
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
  if let Ok(type_name) = results[0].clone().downcast::<String>() {
    // All data flows as Arc<dyn Any + Send + Sync>, so type_name_of_val returns the trait object type
    assert_eq!(
      *type_name,
      "dyn core::any::Any + core::marker::Send + core::marker::Sync"
    );
  } else {
    panic!("Result is not a String");
  }
}

#[tokio::test]
async fn test_type_of_multiple_items() {
  let node = TypeOfNode::new("test_type_of".to_string());
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

  // Verify each result is a string with the trait object type name
  // All data flows as Arc<dyn Any + Send + Sync>, so all type names are the same
  for result in results {
    if let Ok(type_name) = result.clone().downcast::<String>() {
      assert_eq!(
        *type_name,
        "dyn core::any::Any + core::marker::Send + core::marker::Sync"
      );
    } else {
      panic!("Result is not a String");
    }
  }
}
