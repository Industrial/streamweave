//! Tests for RangeNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::range_node::RangeNode;
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
  let (start_tx, start_rx) = mpsc::channel(10);
  let (end_tx, end_rx) = mpsc::channel(10);
  let (step_tx, step_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "start".to_string(),
    Box::pin(ReceiverStream::new(start_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "end".to_string(),
    Box::pin(ReceiverStream::new(end_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "step".to_string(),
    Box::pin(ReceiverStream::new(step_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, start_tx, end_tx, step_tx, inputs)
}

#[tokio::test]
async fn test_range_node_creation() {
  let node = RangeNode::new("test_range".to_string());
  assert_eq!(node.name(), "test_range");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("start"));
  assert!(node.has_input_port("end"));
  assert!(node.has_input_port("step"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
  assert!(!node.has_config());
}

#[tokio::test]
async fn test_range_node_i32_basic() {
  let node = RangeNode::new("test_range".to_string());

  let (_config_tx, start_tx, end_tx, step_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: start=0, end=5, step=1
  let _ = start_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i32) = item.downcast::<i32>() {
            results.push(*arc_i32);
          } else {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results, vec![0, 1, 2, 3, 4]);
}

#[tokio::test]
async fn test_range_node_i32_with_step() {
  let node = RangeNode::new("test_range".to_string());

  let (_config_tx, start_tx, end_tx, step_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: start=0, end=10, step=2
  let _ = start_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i32) = item.downcast::<i32>() {
            results.push(*arc_i32);
          } else {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results, vec![0, 2, 4, 6, 8]);
}

#[tokio::test]
async fn test_range_node_i32_negative_step() {
  let node = RangeNode::new("test_range".to_string());

  let (_config_tx, start_tx, end_tx, step_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: start=10, end=0, step=-2
  let _ = start_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new(-2i32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i32) = item.downcast::<i32>() {
            results.push(*arc_i32);
          } else {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results, vec![10, 8, 6, 4, 2]);
}

#[tokio::test]
async fn test_range_node_i32_empty_range() {
  let node = RangeNode::new("test_range".to_string());

  let (_config_tx, start_tx, end_tx, step_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: start=5, end=5, step=1 (empty range)
  let _ = start_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i32) = item.downcast::<i32>() {
            results.push(*arc_i32);
          } else {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results, Vec::<i32>::new());
}

#[tokio::test]
async fn test_range_node_error_zero_step() {
  let node = RangeNode::new("test_range".to_string());

  let (_config_tx, start_tx, end_tx, step_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: start=0, end=5, step=0 (invalid)
  let _ = start_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect error
  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_string) = item.downcast::<String>() {
            errors.push(arc_string.clone());
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
  assert!(errors[0].contains("Step size cannot be zero"));
}

#[tokio::test]
async fn test_range_node_f64_basic() {
  let node = RangeNode::new("test_range".to_string());

  let (_config_tx, start_tx, end_tx, step_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: start=0.0, end=2.5, step=0.5
  let _ = start_tx
    .send(Arc::new(0.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(2.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new(0.5f64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_f64) = item.downcast::<f64>() {
            results.push(*arc_f64);
          } else {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Check approximate equality for floating point
  assert_eq!(results.len(), 5);
  assert!((results[0] - 0.0).abs() < 0.001);
  assert!((results[1] - 0.5).abs() < 0.001);
  assert!((results[2] - 1.0).abs() < 0.001);
  assert!((results[3] - 1.5).abs() < 0.001);
  assert!((results[4] - 2.0).abs() < 0.001);
}

#[tokio::test]
async fn test_range_node_u32_basic() {
  let node = RangeNode::new("test_range".to_string());

  let (_config_tx, start_tx, end_tx, step_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: start=0, end=10, step=3
  let _ = start_tx
    .send(Arc::new(0u32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(10u32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new(3u32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_u32) = item.downcast::<u32>() {
            results.push(*arc_u32);
          } else {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results, vec![0, 3, 6, 9]);
}

#[tokio::test]
async fn test_range_node_i64_basic() {
  let node = RangeNode::new("test_range".to_string());

  let (_config_tx, start_tx, end_tx, step_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: start=0, end=10, step=3
  let _ = start_tx
    .send(Arc::new(0i64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(10i64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new(3i64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_i64) = item.downcast::<i64>() {
            results.push(*arc_i64);
          } else {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results, vec![0, 3, 6, 9]);
}

#[tokio::test]
async fn test_range_node_f32_basic() {
  let node = RangeNode::new("test_range".to_string());

  let (_config_tx, start_tx, end_tx, step_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: start=0.0, end=2.0, step=0.5
  let _ = start_tx
    .send(Arc::new(0.0f32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(2.0f32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new(0.5f32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_f32) = item.downcast::<f32>() {
            results.push(*arc_f32);
          } else {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Check approximate equality for floating point
  assert_eq!(results.len(), 4);
  assert!((results[0] - 0.0).abs() < 0.001);
  assert!((results[1] - 0.5).abs() < 0.001);
  assert!((results[2] - 1.0).abs() < 0.001);
  assert!((results[3] - 1.5).abs() < 0.001);
}

#[tokio::test]
async fn test_range_node_u64_basic() {
  let node = RangeNode::new("test_range".to_string());

  let (_config_tx, start_tx, end_tx, step_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: start=0, end=12, step=4
  let _ = start_tx
    .send(Arc::new(0u64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(12u64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new(4u64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_u64) = item.downcast::<u64>() {
            results.push(*arc_u64);
          } else {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results, vec![0, 4, 8]);
}

#[tokio::test]
async fn test_range_node_error_unsupported_type() {
  let node = RangeNode::new("test_range".to_string());

  let (_config_tx, start_tx, end_tx, step_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send unsupported type (String)
  let _ = start_tx
    .send(Arc::new("start".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new("end".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new("step".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect error
  let error_stream = outputs.remove("error").unwrap();
  let mut errors = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_string) = item.downcast::<String>() {
            errors.push(arc_string.clone());
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
  assert!(errors[0].contains("Unsupported type"));
}
