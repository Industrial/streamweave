//! Tests for GreaterThanNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::comparison::GreaterThanNode;
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
  let (in1_tx, in1_rx) = mpsc::channel(10);
  let (in2_tx, in2_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in1".to_string(),
    Box::pin(ReceiverStream::new(in1_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in2".to_string(),
    Box::pin(ReceiverStream::new(in2_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in1_tx, in2_tx, inputs)
}

#[tokio::test]
async fn test_greater_than_node_creation() {
  let node = GreaterThanNode::new("test_greater_than".to_string());
  assert_eq!(node.name(), "test_greater_than");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in1"));
  assert!(node.has_input_port("in2"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_greater_than_node_i32_true() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 10 > 5 = true
  let _ = in1_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}

#[tokio::test]
async fn test_greater_than_node_i32_false() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 5 > 10 = false
  let _ = in1_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(!results[0]);
}

#[tokio::test]
async fn test_greater_than_node_i32_equal() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 5 > 5 = false
  let _ = in1_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(!results[0]);
}

#[tokio::test]
async fn test_greater_than_node_f64_true() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 3.5 > 2.5 = true
  let _ = in1_tx
    .send(Arc::new(3.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(2.5f64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}

#[tokio::test]
async fn test_greater_than_node_f64_false() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 2.5 > 3.5 = false
  let _ = in1_tx
    .send(Arc::new(2.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(3.5f64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(!results[0]);
}

#[tokio::test]
async fn test_greater_than_node_type_promotion_i32_to_i64() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: i32 > i64 (10 > 5) = true
  let _ = in1_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(5i64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}

#[tokio::test]
async fn test_greater_than_node_type_promotion_i32_to_f64() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: i32 > f64 (10 > 5.0) = true
  let _ = in1_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(5.0f64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}

#[tokio::test]
async fn test_greater_than_node_incompatible_types() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send incompatible types: String > i32 = false (no error, just returns false)
  let _ = in1_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(!results[0]); // Incompatible types return false
}

#[tokio::test]
async fn test_greater_than_node_u32() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 10 > 5 = true
  let _ = in1_tx
    .send(Arc::new(10u32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(5u32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}

#[tokio::test]
async fn test_greater_than_node_f32() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 3.5 > 2.5 = true
  let _ = in1_tx
    .send(Arc::new(3.5f32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(2.5f32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}

#[tokio::test]
async fn test_greater_than_node_negative_values() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: -5 > -10 = true (negative comparison)
  let _ = in1_tx
    .send(Arc::new(-5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(-10i32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}

#[tokio::test]
async fn test_greater_than_node_i64() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 100 > 50 = true
  let _ = in1_tx
    .send(Arc::new(100i64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(50i64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}

#[tokio::test]
async fn test_greater_than_node_u64() {
  let node = GreaterThanNode::new("test_greater_than".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 100 > 50 = true
  let _ = in1_tx
    .send(Arc::new(100u64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(50u64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_bool) = item.downcast::<bool>() {
            results.push(*arc_bool);
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
  assert!(results[0]);
}
