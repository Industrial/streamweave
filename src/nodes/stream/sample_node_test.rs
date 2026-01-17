//! Tests for SampleNode

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
  let (rate_tx, rate_rx) = mpsc::channel(10);

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
    "rate".to_string(),
    Box::pin(ReceiverStream::new(rate_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, rate_tx, inputs)
}

#[tokio::test]
async fn test_sample_node_creation() {
  let node = SampleNode::new("test_sample".to_string());
  assert_eq!(node.name(), "test_sample");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("rate"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_sample_rate_one() {
  let node = SampleNode::new("test_sample".to_string());

  let (_config_tx, in_tx, rate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send rate: 1 (forward every item)
  let _ = rate_tx
    .send(Arc::new(1usize) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send 5 items
  for i in 1..=5 {
    let _ = in_tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
  }
  drop(in_tx);
  drop(rate_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
          if results.len() == 5 {
            break;
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 5);
  // Verify we got all items
  for (idx, result) in results.iter().enumerate() {
    if let Ok(val) = result.clone().downcast::<i32>() {
      assert_eq!(*val, (idx + 1) as i32);
    } else {
      panic!("Result is not i32");
    }
  }
}

#[tokio::test]
async fn test_sample_rate_two() {
  let node = SampleNode::new("test_sample".to_string());

  let (_config_tx, in_tx, rate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send rate: 2 (forward every 2nd item)
  let _ = rate_tx
    .send(Arc::new(2usize) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send 6 items
  for i in 1..=6 {
    let _ = in_tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
  }
  drop(in_tx);
  drop(rate_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

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
  // Verify we got items 2, 4, 6 (every 2nd item)
  if let (Ok(val1), Ok(val2), Ok(val3)) = (
    results[0].clone().downcast::<i32>(),
    results[1].clone().downcast::<i32>(),
    results[2].clone().downcast::<i32>(),
  ) {
    assert_eq!(*val1, 2);
    assert_eq!(*val2, 4);
    assert_eq!(*val3, 6);
  } else {
    panic!("Results are not i32");
  }
}

#[tokio::test]
async fn test_sample_rate_three() {
  let node = SampleNode::new("test_sample".to_string());

  let (_config_tx, in_tx, rate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send rate: 3 (forward every 3rd item)
  let _ = rate_tx
    .send(Arc::new(3usize) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send 9 items
  for i in 1..=9 {
    let _ = in_tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
  }
  drop(in_tx);
  drop(rate_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

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
  // Verify we got items 3, 6, 9 (every 3rd item)
  if let (Ok(val1), Ok(val2), Ok(val3)) = (
    results[0].clone().downcast::<i32>(),
    results[1].clone().downcast::<i32>(),
    results[2].clone().downcast::<i32>(),
  ) {
    assert_eq!(*val1, 3);
    assert_eq!(*val2, 6);
    assert_eq!(*val3, 9);
  } else {
    panic!("Results are not i32");
  }
}

#[tokio::test]
async fn test_sample_i32_rate() {
  let node = SampleNode::new("test_sample".to_string());

  let (_config_tx, in_tx, rate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send rate: 2 as i32
  let _ = rate_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send 6 items
  for i in 1..=6 {
    let _ = in_tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
  }
  drop(in_tx);
  drop(rate_tx);

  let out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

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
}

#[tokio::test]
async fn test_sample_invalid_rate_zero() {
  let node = SampleNode::new("test_sample".to_string());

  let (_config_tx, in_tx, rate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send invalid rate: zero
  let _ = rate_tx
    .send(Arc::new(0usize) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(rate_tx);
  drop(in_tx);

  // Check error output
  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<String> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            errors.push((*arc_str).clone());
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("positive"));
}

#[tokio::test]
async fn test_sample_invalid_rate_negative() {
  let node = SampleNode::new("test_sample".to_string());

  let (_config_tx, in_tx, rate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send invalid rate: negative value
  let _ = rate_tx
    .send(Arc::new(-5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(rate_tx);
  drop(in_tx);

  // Check error output
  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<String> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            errors.push((*arc_str).clone());
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("positive"));
}

#[tokio::test]
async fn test_sample_invalid_rate_type() {
  let node = SampleNode::new("test_sample".to_string());

  let (_config_tx, in_tx, rate_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future.await.unwrap();

  // Send invalid rate: string instead of numeric
  let _ = rate_tx
    .send(Arc::new("invalid".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(rate_tx);
  drop(in_tx);

  // Check error output
  let error_stream = outputs.remove("error").unwrap();
  let mut errors: Vec<String> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item: Arc<dyn Any + Send + Sync>) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            errors.push((*arc_str).clone());
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(errors.len(), 1);
  assert!(errors[0].contains("numeric") || errors[0].contains("Unsupported"));
}
