//! Tests for PowerNode

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::arithmetic::PowerNode;
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
  let (base_tx, base_rx) = mpsc::channel(10);
  let (exponent_tx, exponent_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "base".to_string(),
    Box::pin(ReceiverStream::new(base_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "exponent".to_string(),
    Box::pin(ReceiverStream::new(exponent_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, base_tx, exponent_tx, inputs)
}

#[tokio::test]
async fn test_power_node_creation() {
  let node = PowerNode::new("test_power".to_string());
  assert_eq!(node.name(), "test_power");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("base"));
  assert!(node.has_input_port("exponent"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_power_node_i32() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 2 ^ 3 = 8
  let _ = base_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0], 8);
}

#[tokio::test]
async fn test_power_node_i32_zero_exponent() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 5 ^ 0 = 1
  let _ = base_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
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
          if let Ok(arc_i64) = item.downcast::<i64>() {
            results.push(*arc_i64);
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
  assert_eq!(results[0], 1);
}

#[tokio::test]
async fn test_power_node_i64() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 3 ^ 4 = 81
  let _ = base_tx
    .send(Arc::new(3i64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
    .send(Arc::new(4i64) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0], 81);
}

#[tokio::test]
async fn test_power_node_f64() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 2.0 ^ 3.0 = 8.0
  let _ = base_tx
    .send(Arc::new(2.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
    .send(Arc::new(3.0f64) as Arc<dyn Any + Send + Sync>)
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
  assert!((results[0] - 8.0).abs() < 0.001);
}

#[tokio::test]
async fn test_power_node_f64_fractional_exponent() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 4.0 ^ 0.5 = 2.0 (square root)
  let _ = base_tx
    .send(Arc::new(4.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
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
  assert!((results[0] - 2.0).abs() < 0.001);
}

#[tokio::test]
async fn test_power_node_f64_negative_exponent() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 2.0 ^ -2.0 = 0.25 (negative exponent allowed for floats)
  let _ = base_tx
    .send(Arc::new(2.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
    .send(Arc::new(-2.0f64) as Arc<dyn Any + Send + Sync>)
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
  assert!((results[0] - 0.25).abs() < 0.001);
}

#[tokio::test]
async fn test_power_node_type_promotion_i32_to_i64() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: i32 ^ i64 -> i64
  let _ = base_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
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
  assert_eq!(results[0], 8);
}

#[tokio::test]
async fn test_power_node_type_promotion_i32_to_f32() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: i32 ^ f32 -> f32
  let _ = base_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
    .send(Arc::new(3.0f32) as Arc<dyn Any + Send + Sync>)
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
  assert!((results[0] - 8.0).abs() < 0.001);
}

#[tokio::test]
async fn test_power_node_negative_exponent_integer() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 2 ^ -3 (negative exponent not allowed for integers)
  let _ = base_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
    .send(Arc::new(-3i32) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("Negative exponent not supported"));
}

#[tokio::test]
async fn test_power_node_overflow() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values that will overflow i64: 1000000 ^ 10 (very large result)
  let _ = base_tx
    .send(Arc::new(1000000i64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
    .send(Arc::new(10i64) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("overflow"));
}

#[tokio::test]
async fn test_power_node_unsupported_type() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send unsupported types (String)
  let _ = base_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
    .send(Arc::new("world".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("Unsupported types"));
}

#[tokio::test]
async fn test_power_node_u32() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 3 ^ 2 = 9
  let _ = base_tx
    .send(Arc::new(3u32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
    .send(Arc::new(2u32) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0], 9);
}

#[tokio::test]
async fn test_power_node_u64() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 2 ^ 4 = 16
  let _ = base_tx
    .send(Arc::new(2u64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
    .send(Arc::new(4u32) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0], 16);
}

#[tokio::test]
async fn test_power_node_f32() {
  let node = PowerNode::new("test_power".to_string());

  let (_config_tx, base_tx, exponent_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 2.0 ^ 3.0 = 8.0
  let _ = base_tx
    .send(Arc::new(2.0f32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = exponent_tx
    .send(Arc::new(3.0f32) as Arc<dyn Any + Send + Sync>)
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
  assert!((results[0] - 8.0).abs() < 0.001);
}
