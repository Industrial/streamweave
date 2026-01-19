//! Tests for DivideNode

use crate::node::{InputStreams, Node};
use crate::nodes::arithmetic::DivideNode;
use crate::nodes::common::TestSender;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams() -> (TestSender, TestSender, TestSender, InputStreams) {
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
async fn test_divide_node_creation() {
  let node = DivideNode::new("test_divide".to_string());
  assert_eq!(node.name(), "test_divide");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in1"));
  assert!(node.has_input_port("in2"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_divide_node_i32() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 20 / 4 = 5
  let _ = in1_tx
    .send(Arc::new(20i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(4i32) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0], 5);
}

#[tokio::test]
async fn test_divide_node_i32_truncation() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 7 / 3 = 2 (integer division truncates)
  let _ = in1_tx
    .send(Arc::new(7i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
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
          if let Ok(arc_i32) = item.downcast::<i32>() {
            results.push(*arc_i32);
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
  assert_eq!(results[0], 2); // Integer division truncates
}

#[tokio::test]
async fn test_divide_node_i64() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 100 / 5 = 20
  let _ = in1_tx
    .send(Arc::new(100i64) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0], 20);
}

#[tokio::test]
async fn test_divide_node_f64() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 10.0 / 4.0 = 2.5
  let _ = in1_tx
    .send(Arc::new(10.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(4.0f64) as Arc<dyn Any + Send + Sync>)
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
  assert!((results[0] - 2.5).abs() < 0.001);
}

#[tokio::test]
async fn test_divide_node_type_promotion_i32_to_i64() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: i32 / i64 -> i64
  let _ = in1_tx
    .send(Arc::new(20i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
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
  assert_eq!(results[0], 5);
}

#[tokio::test]
async fn test_divide_node_type_promotion_i32_to_f32() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: i32 / f32 -> f32
  let _ = in1_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(4.0f32) as Arc<dyn Any + Send + Sync>)
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
  assert!((results[0] - 2.5).abs() < 0.001);
}

#[tokio::test]
async fn test_divide_node_division_by_zero_i32() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 10 / 0 (division by zero)
  let _ = in1_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
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
  assert!(errors[0].contains("Division by zero"));
}

#[tokio::test]
async fn test_divide_node_division_by_zero_f64() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 10.0 / 0.0 (division by zero)
  let _ = in1_tx
    .send(Arc::new(10.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(0.0f64) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("Division by zero"));
}

#[tokio::test]
async fn test_divide_node_unsupported_type() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send unsupported types (String)
  let _ = in1_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
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
async fn test_divide_node_u32() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 12 / 3 = 4
  let _ = in1_tx
    .send(Arc::new(12u32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
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
  assert_eq!(results[0], 4);
}

#[tokio::test]
async fn test_divide_node_u64() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 100 / 5 = 20
  let _ = in1_tx
    .send(Arc::new(100u64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(5u64) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0], 20);
}

#[tokio::test]
async fn test_divide_node_f32() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: 10.0 / 4.0 = 2.5
  let _ = in1_tx
    .send(Arc::new(10.0f32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(4.0f32) as Arc<dyn Any + Send + Sync>)
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
  assert!((results[0] - 2.5).abs() < 0.001);
}

#[tokio::test]
async fn test_divide_node_negative_result() {
  let node = DivideNode::new("test_divide".to_string());

  let (_config_tx, in1_tx, in2_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: -20 / 4 = -5 (negative result for signed integers)
  let _ = in1_tx
    .send(Arc::new(-20i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(4i32) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(results[0], -5);
}
