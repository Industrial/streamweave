//! Tests for ExpNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::math::ExpNode;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels for single input nodes
fn create_input_streams() -> (
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, inputs)
}

#[tokio::test]
async fn test_exp_node_creation() {
  let node = ExpNode::new("test_exp".to_string());
  assert_eq!(node.name(), "test_exp");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_exp_node_zero() {
  let node = ExpNode::new("test_exp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send value: e^0 = 1.0
  let _ = in_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
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
  assert!((results[0] - 1.0).abs() < 1e-10);
}

#[tokio::test]
async fn test_exp_node_one() {
  let node = ExpNode::new("test_exp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send value: e^1 = e ≈ 2.718281828
  let _ = in_tx
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
          if let Ok(arc_f64) = item.downcast::<f64>() {
            results.push(*arc_f64);
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
  assert!((results[0] - std::f64::consts::E).abs() < 1e-10);
}

#[tokio::test]
async fn test_exp_node_i32() {
  let node = ExpNode::new("test_exp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send value: e^2 ≈ 7.389
  let _ = in_tx
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
          if let Ok(arc_f64) = item.downcast::<f64>() {
            results.push(*arc_f64);
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
  let expected = 2_f64.exp();
  assert!((results[0] - expected).abs() < 1e-10);
}

#[tokio::test]
async fn test_exp_node_i64() {
  let node = ExpNode::new("test_exp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send value: e^3 ≈ 20.085
  let _ = in_tx
    .send(Arc::new(3i64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_f64) = item.downcast::<f64>() {
            results.push(*arc_f64);
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
  let expected = 3_f64.exp();
  assert!((results[0] - expected).abs() < 1e-10);
}

#[tokio::test]
async fn test_exp_node_u32() {
  let node = ExpNode::new("test_exp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send value: e^1 ≈ 2.718
  let _ = in_tx
    .send(Arc::new(1u32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_f64) = item.downcast::<f64>() {
            results.push(*arc_f64);
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
  let expected = 1_f64.exp();
  assert!((results[0] - expected).abs() < 1e-10);
}

#[tokio::test]
async fn test_exp_node_u64() {
  let node = ExpNode::new("test_exp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send value: e^0 = 1.0
  let _ = in_tx
    .send(Arc::new(0u64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_f64) = item.downcast::<f64>() {
            results.push(*arc_f64);
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
  assert!((results[0] - 1.0).abs() < 1e-10);
}

#[tokio::test]
async fn test_exp_node_f32() {
  let node = ExpNode::new("test_exp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send value: e^0.5 ≈ 1.649
  let _ = in_tx
    .send(Arc::new(0.5f32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_f64) = item.downcast::<f64>() {
            results.push(*arc_f64);
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
  // Node converts f32 to f64 first, then computes exp
  let expected = (0.5f32 as f64).exp();
  assert!((results[0] - expected).abs() < 1e-10);
}

#[tokio::test]
async fn test_exp_node_f64() {
  let node = ExpNode::new("test_exp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send value: e^2.0 ≈ 7.389
  let _ = in_tx
    .send(Arc::new(2.0f64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_f64) = item.downcast::<f64>() {
            results.push(*arc_f64);
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
  let expected = 2.0f64.exp();
  assert!((results[0] - expected).abs() < 1e-10);
}

#[tokio::test]
async fn test_exp_node_negative() {
  let node = ExpNode::new("test_exp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send value: e^(-1) ≈ 0.368
  let _ = in_tx
    .send(Arc::new(-1i32) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_f64) = item.downcast::<f64>() {
            results.push(*arc_f64);
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
  let expected = (-1_f64).exp();
  assert!((results[0] - expected).abs() < 1e-10);
  assert!(results[0] > 0.0); // Should be positive
}

#[tokio::test]
async fn test_exp_node_f64_negative() {
  let node = ExpNode::new("test_exp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send value: e^(-2.0) ≈ 0.135
  let _ = in_tx
    .send(Arc::new(-2.0f64) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_f64) = item.downcast::<f64>() {
            results.push(*arc_f64);
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
  let expected = (-2.0f64).exp();
  assert!((results[0] - expected).abs() < 1e-10);
}

#[tokio::test]
async fn test_exp_node_unsupported_type() {
  let node = ExpNode::new("test_exp".to_string());

  let (_config_tx, in_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send unsupported type (String)
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect errors
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
  assert!(errors[0].contains("Unsupported type for exponential"));
}
