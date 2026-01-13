//! Tests for ArraySliceNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::array::ArraySliceNode;
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
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (start_tx, start_rx) = mpsc::channel(10);
  let (end_tx, end_rx) = mpsc::channel(10);

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
    "start".to_string(),
    Box::pin(ReceiverStream::new(start_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "end".to_string(),
    Box::pin(ReceiverStream::new(end_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, start_tx, end_tx, inputs)
}

#[tokio::test]
async fn test_array_slice_node_creation() {
  let node = ArraySliceNode::new("test_slice".to_string());
  assert_eq!(node.name(), "test_slice");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("start"));
  assert!(node.has_input_port("end"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_array_slice_basic() {
  let node = ArraySliceNode::new("test_slice".to_string());

  let (_config_tx, in_tx, start_tx, end_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3, 4, 5] with start=1, end=4 → [2, 3, 4]
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
  let _ = start_tx
    .send(Arc::new(1usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(4usize) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results
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
  assert_eq!(results[0].len(), 3);
  // Verify elements
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  let elem1 = results[0][1].clone().downcast::<i32>().unwrap();
  let elem2 = results[0][2].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem0, 2i32);
  assert_eq!(*elem1, 3i32);
  assert_eq!(*elem2, 4i32);
}

#[tokio::test]
async fn test_array_slice_from_start() {
  let node = ArraySliceNode::new("test_slice".to_string());

  let (_config_tx, in_tx, start_tx, end_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3] with start=0, end=2 → [1, 2]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = start_tx
    .send(Arc::new(0usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(2usize) as Arc<dyn Any + Send + Sync>)
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
async fn test_array_slice_to_end() {
  let node = ArraySliceNode::new("test_slice".to_string());

  let (_config_tx, in_tx, start_tx, end_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3, 4] with start=2, end=4 → [3, 4]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(4i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = start_tx
    .send(Arc::new(2usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(4usize) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(*elem0, 3i32);
  assert_eq!(*elem1, 4i32);
}

#[tokio::test]
async fn test_array_slice_empty() {
  let node = ArraySliceNode::new("test_slice".to_string());

  let (_config_tx, in_tx, start_tx, end_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3] with start=1, end=1 → [] (empty slice)
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = start_tx
    .send(Arc::new(1usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(1usize) as Arc<dyn Any + Send + Sync>)
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
async fn test_array_slice_out_of_bounds() {
  let node = ArraySliceNode::new("test_slice".to_string());

  let (_config_tx, in_tx, start_tx, end_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2] with start=0, end=10 → [1, 2] (clamped to bounds)
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = start_tx
    .send(Arc::new(0usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(10usize) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 2); // Clamped to array length
}

#[tokio::test]
async fn test_array_slice_invalid_range() {
  let node = ArraySliceNode::new("test_slice".to_string());

  let (_config_tx, in_tx, start_tx, end_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3] with start=2, end=1 → error (start > end)
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = start_tx
    .send(Arc::new(2usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(1usize) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("start index") && errors[0].contains("greater than"));
}

#[tokio::test]
async fn test_array_slice_non_array_input() {
  let node = ArraySliceNode::new("test_slice".to_string());

  let (_config_tx, in_tx, start_tx, end_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-array input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = start_tx
    .send(Arc::new(0usize) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(1usize) as Arc<dyn Any + Send + Sync>)
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
async fn test_array_slice_numeric_index_types() {
  let node = ArraySliceNode::new("test_slice".to_string());

  let (_config_tx, in_tx, start_tx, end_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [1, 2, 3, 4] with start=1 (as i32), end=3 (as i32) → [2, 3]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(4i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = start_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(*elem0, 2i32);
  assert_eq!(*elem1, 3i32);
}
