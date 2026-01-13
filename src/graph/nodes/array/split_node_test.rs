//! Tests for ArraySplitNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::array::ArraySplitNode;
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
  let (in_tx, in_rx) = mpsc::channel(10);
  let (chunk_size_tx, chunk_size_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "chunk_size".to_string(),
    Box::pin(ReceiverStream::new(chunk_size_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, chunk_size_tx, inputs)
}

#[tokio::test]
async fn test_array_split_node_creation() {
  let node = ArraySplitNode::new("test_split".to_string());
  assert_eq!(node.name(), "test_split");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("chunk_size"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_array_split_basic() {
  let node = ArraySplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, chunk_size_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3, 4, 5] with chunk_size 2 → [[1, 2], [3, 4], [5]]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(4i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(5i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = chunk_size_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 3); // Three chunks

  // Verify first chunk [1, 2]
  if let Ok(chunk1) = results[0][0]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(chunk1.len(), 2);
    let elem0 = chunk1[0].clone().downcast::<i32>().unwrap();
    let elem1 = chunk1[1].clone().downcast::<i32>().unwrap();
    assert_eq!(*elem0, 1i32);
    assert_eq!(*elem1, 2i32);
  } else {
    panic!("First chunk is not a Vec");
  }

  // Verify second chunk [3, 4]
  if let Ok(chunk2) = results[0][1]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(chunk2.len(), 2);
    let elem0 = chunk2[0].clone().downcast::<i32>().unwrap();
    let elem1 = chunk2[1].clone().downcast::<i32>().unwrap();
    assert_eq!(*elem0, 3i32);
    assert_eq!(*elem1, 4i32);
  } else {
    panic!("Second chunk is not a Vec");
  }

  // Verify third chunk [5]
  if let Ok(chunk3) = results[0][2]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(chunk3.len(), 1);
    let elem0 = chunk3[0].clone().downcast::<i32>().unwrap();
    assert_eq!(*elem0, 5i32);
  } else {
    panic!("Third chunk is not a Vec");
  }
}

#[tokio::test]
async fn test_array_split_empty() {
  let node = ArraySplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, chunk_size_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [] with chunk_size 2 → []
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = chunk_size_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 0); // No chunks
}

#[tokio::test]
async fn test_array_split_exact_divisor() {
  let node = ArraySplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, chunk_size_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3, 4] with chunk_size 2 → [[1, 2], [3, 4]]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(4i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = chunk_size_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 2); // Two chunks
}

#[tokio::test]
async fn test_array_split_chunk_size_one() {
  let node = ArraySplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, chunk_size_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3] with chunk_size 1 → [[1], [2], [3]]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = chunk_size_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 3); // Three chunks, each with one element
}

#[tokio::test]
async fn test_array_split_large_chunk_size() {
  let node = ArraySplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, chunk_size_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3] with chunk_size 10 → [[1, 2, 3]]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = chunk_size_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 1); // One chunk with all elements
}

#[tokio::test]
async fn test_array_split_non_array_input() {
  let node = ArraySplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, chunk_size_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-array input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = chunk_size_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
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
async fn test_array_split_zero_chunk_size() {
  let node = ArraySplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, chunk_size_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3] with chunk_size 0 → error
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = chunk_size_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("chunk_size must be greater than 0"));
}

#[tokio::test]
async fn test_array_split_negative_chunk_size() {
  let node = ArraySplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, chunk_size_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3] with chunk_size -1 → error
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = chunk_size_tx
    .send(Arc::new(-1i32) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("must be non-negative"));
}
