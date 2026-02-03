//! Tests for ToArrayNode

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::type_ops::ToArrayNode;
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
async fn test_to_array_node_creation() {
  let node = ToArrayNode::new("test_to_array".to_string());
  assert_eq!(node.name(), "test_to_array");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_to_array_array() {
  let node = ToArrayNode::new("test_to_array".to_string());
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
  if let Ok(result_arc) = results[0]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    let result_array = &*result_arc;
    assert_eq!(result_array.len(), 2);
    if let Ok(val1) = result_array[0].clone().downcast::<i32>() {
      assert_eq!(*val1, 1);
    } else {
      panic!("First element is not i32");
    }
    if let Ok(val2) = result_array[1].clone().downcast::<i32>() {
      assert_eq!(*val2, 2);
    } else {
      panic!("Second element is not i32");
    }
  } else {
    panic!("Result is not an array");
  }
}

#[tokio::test]
async fn test_to_array_string() {
  let node = ToArrayNode::new("test_to_array".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send a string
  let _ = in_tx
    .send(Arc::new("hi".to_string()) as Arc<dyn Any + Send + Sync>)
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
  if let Ok(result_array) = results[0]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(result_array.len(), 2);
    if let Ok(char1) = result_array[0].clone().downcast::<String>() {
      assert_eq!(*char1, "h");
    } else {
      panic!("First element is not a String");
    }
    if let Ok(char2) = result_array[1].clone().downcast::<String>() {
      assert_eq!(*char2, "i");
    } else {
      panic!("Second element is not a String");
    }
  } else {
    panic!("Result is not an array");
  }
}

#[tokio::test]
async fn test_to_array_i32() {
  let node = ToArrayNode::new("test_to_array".to_string());
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
  if let Ok(result_array) = results[0]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(result_array.len(), 1);
    if let Ok(num_val) = result_array[0].clone().downcast::<i32>() {
      assert_eq!(*num_val, 42);
    } else {
      panic!("Element is not i32");
    }
  } else {
    panic!("Result is not an array");
  }
}

#[tokio::test]
async fn test_to_array_multiple_types() {
  let node = ToArrayNode::new("test_to_array".to_string());
  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send multiple items of different types
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new("hi".to_string()) as Arc<dyn Any + Send + Sync>)
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

  // First item: i32 -> [42]
  if let Ok(result_array) = results[0]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(result_array.len(), 1);
    if let Ok(num_val) = result_array[0].clone().downcast::<i32>() {
      assert_eq!(*num_val, 42);
    } else {
      panic!("First result element is not i32");
    }
  } else {
    panic!("First result is not an array");
  }

  // Second item: String "hi" -> ["h", "i"]
  if let Ok(result_array) = results[1]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(result_array.len(), 2);
    if let Ok(char1) = result_array[0].clone().downcast::<String>() {
      assert_eq!(*char1, "h");
    } else {
      panic!("Second result first element is not String");
    }
    if let Ok(char2) = result_array[1].clone().downcast::<String>() {
      assert_eq!(*char2, "i");
    } else {
      panic!("Second result second element is not String");
    }
  } else {
    panic!("Second result is not an array");
  }

  // Third item: bool true -> [true]
  if let Ok(result_array) = results[2]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(result_array.len(), 1);
    if let Ok(bool_val) = result_array[0].clone().downcast::<bool>() {
      assert!(*bool_val);
    } else {
      panic!("Third result element is not bool");
    }
  } else {
    panic!("Third result is not an array");
  }
}
