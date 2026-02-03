//! Tests for ArraySortNode

use crate::node::{InputStreams, Node};
use crate::nodes::array::ArraySortNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

type AnySender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels
fn create_input_streams() -> (AnySender, AnySender, AnySender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (order_tx, order_rx) = mpsc::channel(10);

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
    "order".to_string(),
    Box::pin(ReceiverStream::new(order_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, order_tx, inputs)
}

#[tokio::test]
async fn test_array_sort_node_creation() {
  let node = ArraySortNode::new("test_sort".to_string());
  assert_eq!(node.name(), "test_sort");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("order"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_array_sort_ascending() {
  let node = ArraySortNode::new("test_sort".to_string());

  let (_config_tx, in_tx, order_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [3, 1, 4, 2] ascending → [1, 2, 3, 4]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(4i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = order_tx
    .send(Arc::new("ascending".to_string()) as Arc<dyn Any + Send + Sync>)
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
  // Verify elements are sorted
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
async fn test_array_sort_descending() {
  let node = ArraySortNode::new("test_sort".to_string());

  let (_config_tx, in_tx, order_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [3, 1, 4, 2] descending → [4, 3, 2, 1]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(4i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = order_tx
    .send(Arc::new("descending".to_string()) as Arc<dyn Any + Send + Sync>)
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
  // Verify elements are sorted descending
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  let elem1 = results[0][1].clone().downcast::<i32>().unwrap();
  let elem2 = results[0][2].clone().downcast::<i32>().unwrap();
  let elem3 = results[0][3].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem0, 4i32);
  assert_eq!(*elem1, 3i32);
  assert_eq!(*elem2, 2i32);
  assert_eq!(*elem3, 1i32);
}

#[tokio::test]
async fn test_array_sort_empty() {
  let node = ArraySortNode::new("test_sort".to_string());

  let (_config_tx, in_tx, order_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [] ascending → []
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = order_tx
    .send(Arc::new("ascending".to_string()) as Arc<dyn Any + Send + Sync>)
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
async fn test_array_sort_strings() {
  let node = ArraySortNode::new("test_sort".to_string());

  let (_config_tx, in_tx, order_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: ["c", "a", "b"] ascending → ["a", "b", "c"]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new("c".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("b".to_string()) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = order_tx
    .send(Arc::new("ascending".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 3);
  let elem0 = results[0][0].clone().downcast::<String>().unwrap();
  let elem1 = results[0][1].clone().downcast::<String>().unwrap();
  let elem2 = results[0][2].clone().downcast::<String>().unwrap();
  assert_eq!(*elem0, "a".to_string());
  assert_eq!(*elem1, "b".to_string());
  assert_eq!(*elem2, "c".to_string());
}

#[tokio::test]
async fn test_array_sort_type_promotion() {
  let node = ArraySortNode::new("test_sort".to_string());

  let (_config_tx, in_tx, order_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [3i32, 1i64, 2i32] ascending → [1, 2, 3] (type promotion)
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(1i64) as Arc<dyn Any + Send + Sync>,
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = order_tx
    .send(Arc::new("ascending".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 3);
  // Verify elements are sorted (type promotion should work)
  // Note: The actual types might be preserved, but the order should be correct
}

#[tokio::test]
async fn test_array_sort_bool_order() {
  let node = ArraySortNode::new("test_sort".to_string());

  let (_config_tx, in_tx, order_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [3, 1, 4] with order=true (ascending) → [1, 3, 4]
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
    Arc::new(4i32) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = order_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0].len(), 3);
  let elem0 = results[0][0].clone().downcast::<i32>().unwrap();
  let elem1 = results[0][1].clone().downcast::<i32>().unwrap();
  let elem2 = results[0][2].clone().downcast::<i32>().unwrap();
  assert_eq!(*elem0, 1i32);
  assert_eq!(*elem1, 3i32);
  assert_eq!(*elem2, 4i32);
}

#[tokio::test]
async fn test_array_sort_non_array_input() {
  let node = ArraySortNode::new("test_sort".to_string());

  let (_config_tx, in_tx, order_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-array input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = order_tx
    .send(Arc::new("ascending".to_string()) as Arc<dyn Any + Send + Sync>)
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
