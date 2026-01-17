//! Tests for ForEachNode

use crate::node::{InputStreams, Node};
use crate::nodes::for_each_node::{ForEachConfig, ForEachNode, for_each_config};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

type ConfigSender = mpsc::Sender<Arc<dyn Any + Send + Sync>>;

/// Helper to create input streams from channels
fn create_input_streams() -> (
  ConfigSender,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (data_tx, data_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as crate::node::InputStream,
  );

  (config_tx, data_tx, inputs)
}

#[tokio::test]
async fn test_for_each_node_creation() {
  let node = ForEachNode::new("test_for_each".to_string());
  assert_eq!(node.name(), "test_for_each");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
  assert!(!node.has_config());
}

#[tokio::test]
async fn test_for_each_node_vec_i32() {
  let node = ForEachNode::new("test_for_each".to_string());
  let config: ForEachConfig = for_each_config(|value| async move {
    if let Ok(arc_vec) = value.downcast::<Vec<i32>>() {
      let items: Vec<Arc<dyn Any + Send + Sync>> = arc_vec
        .iter()
        .map(|&n| Arc::new(n) as Arc<dyn Any + Send + Sync>)
        .collect();
      Ok(items)
    } else {
      Err("Expected Vec<i32>".to_string())
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data
  let vec_data = vec![1, 2, 3, 4, 5];
  let _ = data_tx
    .send(Arc::new(vec_data) as Arc<dyn Any + Send + Sync>)
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
            if results.len() >= 5 {
              break;
            }
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 5);
  assert_eq!(results, vec![1, 2, 3, 4, 5]);
}

#[tokio::test]
async fn test_for_each_node_empty_collection() {
  let node = ForEachNode::new("test_for_each".to_string());
  let config: ForEachConfig = for_each_config(|value| async move {
    if let Ok(arc_vec) = value.downcast::<Vec<i32>>() {
      let items: Vec<Arc<dyn Any + Send + Sync>> = arc_vec
        .iter()
        .map(|&n| Arc::new(n) as Arc<dyn Any + Send + Sync>)
        .collect();
      Ok(items)
    } else {
      Err("Expected Vec<i32>".to_string())
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send empty collection
  let vec_data: Vec<i32> = vec![];
  let _ = data_tx
    .send(Arc::new(vec_data) as Arc<dyn Any + Send + Sync>)
    .await;

  // Collect results - should be empty
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
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_for_each_node_vec_string() {
  let node = ForEachNode::new("test_for_each".to_string());
  let config: ForEachConfig = for_each_config(|value| async move {
    if let Ok(arc_vec) = value.downcast::<Vec<String>>() {
      let items: Vec<Arc<dyn Any + Send + Sync>> = arc_vec
        .iter()
        .map(|s| Arc::new(s.clone()) as Arc<dyn Any + Send + Sync>)
        .collect();
      Ok(items)
    } else {
      Err("Expected Vec<String>".to_string())
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data
  let vec_data = vec!["hello".to_string(), "world".to_string(), "test".to_string()];
  let _ = data_tx
    .send(Arc::new(vec_data) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_string) = item.downcast::<String>() {
            results.push(arc_string.clone());
            if results.len() >= 3 {
              break;
            }
          }
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 3);
  assert_eq!(&*results[0], "hello");
  assert_eq!(&*results[1], "world");
  assert_eq!(&*results[2], "test");
}

#[tokio::test]
async fn test_for_each_node_error_on_invalid_type() {
  let node = ForEachNode::new("test_for_each".to_string());
  let config: ForEachConfig = for_each_config(|value| async move {
    if let Ok(_arc_vec) = value.downcast::<Vec<i32>>() {
      Ok(vec![])
    } else {
      Err("Expected Vec<i32>".to_string())
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send invalid data (String instead of Vec<i32>)
  let _ = data_tx
    .send(Arc::new("invalid".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("Expected Vec<i32>"));
}

#[tokio::test]
async fn test_for_each_node_error_on_no_config() {
  let node = ForEachNode::new("test_for_each".to_string());

  let (_config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send data without configuration
  let vec_data = vec![1, 2, 3];
  let _ = data_tx
    .send(Arc::new(vec_data) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("No configuration set"));
}
