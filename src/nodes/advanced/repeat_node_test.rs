//! Tests for RepeatNode

use crate::node::InputStreams;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Helper to create input streams from channels
fn create_input_streams() -> (
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (count_tx, count_rx) = mpsc::channel(10);

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
    "count".to_string(),
    Box::pin(ReceiverStream::new(count_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, count_tx, inputs)
}

#[tokio::test]
async fn test_repeat_node_creation() {
  use crate::nodes::advanced::RepeatNode;
  let node = RepeatNode::new("test_repeat".to_string());
  assert_eq!(node.name(), "test_repeat");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("count"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_repeat_basic() {
  use crate::nodes::advanced::RepeatNode;
  let node = RepeatNode::new("test_repeat".to_string());

  let (_config_tx, in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send count first
  let _ = count_tx
    .send(Arc::new(3usize) as Arc<dyn Any + Send + Sync>)
    .await;
  // Send an item
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(count_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
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
  for result in results {
    if let Ok(num_val) = result.downcast::<i32>() {
      assert_eq!(*num_val, 42);
    } else {
      panic!("Result is not i32");
    }
  }
}

#[tokio::test]
async fn test_repeat_multiple_items() {
  use crate::nodes::advanced::RepeatNode;
  let node = RepeatNode::new("test_repeat".to_string());

  let (_config_tx, in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send count first
  let _ = count_tx
    .send(Arc::new(2usize) as Arc<dyn Any + Send + Sync>)
    .await;
  // Send multiple items
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(count_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
          if results.len() == 4 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 4);
  // First two should be 1, next two should be 2
  if let Ok(num_val) = results[0].clone().downcast::<i32>() {
    assert_eq!(*num_val, 1);
  } else {
    panic!("First result is not i32");
  }
  if let Ok(num_val) = results[1].clone().downcast::<i32>() {
    assert_eq!(*num_val, 1);
  } else {
    panic!("Second result is not i32");
  }
  if let Ok(num_val) = results[2].clone().downcast::<i32>() {
    assert_eq!(*num_val, 2);
  } else {
    panic!("Third result is not i32");
  }
  if let Ok(num_val) = results[3].clone().downcast::<i32>() {
    assert_eq!(*num_val, 2);
  } else {
    panic!("Fourth result is not i32");
  }
}

#[tokio::test]
async fn test_repeat_zero_count() {
  use crate::nodes::advanced::RepeatNode;
  let node = RepeatNode::new("test_repeat".to_string());

  let (_config_tx, in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send count of 0
  let _ = count_tx
    .send(Arc::new(0usize) as Arc<dyn Any + Send + Sync>)
    .await;
  // Send an item
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(count_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should have no results (0 repeats)
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_repeat_buffered_items() {
  use crate::nodes::advanced::RepeatNode;
  let node = RepeatNode::new("test_repeat".to_string());

  let (_config_tx, in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send items before count (should be buffered)
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  // Now send count
  let _ = count_tx
    .send(Arc::new(2usize) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(count_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
          if results.len() == 4 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 4);
  // First two should be 1, next two should be 2
  if let Ok(num_val) = results[0].clone().downcast::<i32>() {
    assert_eq!(*num_val, 1);
  } else {
    panic!("First result is not i32");
  }
  if let Ok(num_val) = results[1].clone().downcast::<i32>() {
    assert_eq!(*num_val, 1);
  } else {
    panic!("Second result is not i32");
  }
  if let Ok(num_val) = results[2].clone().downcast::<i32>() {
    assert_eq!(*num_val, 2);
  } else {
    panic!("Third result is not i32");
  }
  if let Ok(num_val) = results[3].clone().downcast::<i32>() {
    assert_eq!(*num_val, 2);
  } else {
    panic!("Fourth result is not i32");
  }
}

#[tokio::test]
async fn test_repeat_invalid_count_type() {
  use crate::nodes::advanced::RepeatNode;
  let node = RepeatNode::new("test_repeat".to_string());

  let (_config_tx, _in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid count type (String)
  let _ = count_tx
    .send(Arc::new("invalid".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(count_tx);

  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          errors.push(item);
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  if let Ok(err_str) = errors[0].clone().downcast::<String>() {
    assert!(err_str.contains("Unsupported type for count"));
  } else {
    panic!("Error is not a String");
  }
}

#[tokio::test]
async fn test_repeat_negative_count() {
  use crate::nodes::advanced::RepeatNode;
  let node = RepeatNode::new("test_repeat".to_string());

  let (_config_tx, _in_tx, count_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send negative count
  let _ = count_tx
    .send(Arc::new(-1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(count_tx);

  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          errors.push(item);
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  if let Ok(err_str) = errors[0].clone().downcast::<String>() {
    assert!(err_str.contains("cannot be negative"));
  } else {
    panic!("Error is not a String");
  }
}
