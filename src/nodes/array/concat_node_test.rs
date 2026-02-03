//! Tests for ArrayConcatNode

use crate::node::{InputStreams, Node};
use crate::nodes::array::ArrayConcatNode;
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
async fn test_array_concat_node_creation() {
  let node = ArrayConcatNode::new("test_concat".to_string());
  assert_eq!(node.name(), "test_concat");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in1"));
  assert!(node.has_input_port("in2"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_array_concat_basic() {
  let node = ArrayConcatNode::new("test_concat".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2] + [3, 4] → [1, 2, 3, 4]
  let vec1: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let vec2: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(4i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in1_tx
    .send(Arc::new(vec1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(vec2) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 4);
  // Verify elements
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  let elem1 = results[0][1].clone().downcast::<i32>().unwrap();
  let elem2 = results[0][2].clone().downcast::<i32>().unwrap();
  let elem3 = results[0][3].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem0, 1i32);
  assert_eq!(*elem1, 2i32);
  assert_eq!(*elem2, 3i32);
  assert_eq!(*elem3, 4i32);
}

#[tokio::test]
async fn test_array_concat_empty_first() {
  let node = ArrayConcatNode::new("test_concat".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [] + [1, 2] → [1, 2]
  let vec1: Vec<Arc<dyn Any + Send + Sync>> = vec![];
  let vec2: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in1_tx
    .send(Arc::new(vec1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(vec2) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 2);
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  let elem1 = results[0][1].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem0, 1i32);
  assert_eq!(*elem1, 2i32);
}

#[tokio::test]
async fn test_array_concat_empty_second() {
  let node = ArrayConcatNode::new("test_concat".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2] + [] → [1, 2]
  let vec1: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let vec2: Vec<Arc<dyn Any + Send + Sync>> = vec![];
  let _ = in1_tx
    .send(Arc::new(vec1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(vec2) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 2);
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  let elem1 = results[0][1].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem0, 1i32);
  assert_eq!(*elem1, 2i32);
}

#[tokio::test]
async fn test_array_concat_both_empty() {
  let node = ArrayConcatNode::new("test_concat".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [] + [] → []
  let vec1: Vec<Arc<dyn Any + Send + Sync>> = vec![];
  let vec2: Vec<Arc<dyn Any + Send + Sync>> = vec![];
  let _ = in1_tx
    .send(Arc::new(vec1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(vec2) as Arc<dyn Any + Send + Sync>)
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
async fn test_array_concat_mixed_types() {
  let node = ArrayConcatNode::new("test_concat".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, "hello"] + [true, 2] → [1, "hello", true, 2]
  let vec1: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>,
  ];
  let vec2: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(true) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in1_tx
    .send(Arc::new(vec1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(vec2) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 4);
  // Verify first element is i32
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem0, 1i32);
  // Verify second element is String
  let elem1 = results[0][1].clone().downcast::<String>().unwrap();
  assert_eq!(*elem1, "hello".to_string());
  // Verify third element is bool
  let elem2 = results[0][2].clone().downcast::<bool>().unwrap();
  assert!(*elem2);
  // Verify fourth element is i32
  let elem3 = results[0][3].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem3, 2i32);
}

#[tokio::test]
async fn test_array_concat_non_array_input1() {
  let node = ArrayConcatNode::new("test_concat".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-array input for in1
  let vec2: Vec<Arc<dyn Any + Send + Sync>> = vec![Arc::new(1i32) as Arc<dyn Any + Send + Sync>];
  let _ = in1_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(vec2) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("input must be Vec"));
}

#[tokio::test]
async fn test_array_concat_non_array_input2() {
  let node = ArrayConcatNode::new("test_concat".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-array input for in2
  let vec1: Vec<Arc<dyn Any + Send + Sync>> = vec![Arc::new(1i32) as Arc<dyn Any + Send + Sync>];
  let _ = in1_tx
    .send(Arc::new(vec1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
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
  assert!(errors[0].contains("input must be Vec"));
}
