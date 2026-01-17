//! Tests for StringSplitNode

use crate::node::{InputStreams, Node};
use crate::nodes::string::StringSplitNode;
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
  let (in_tx, in_rx) = mpsc::channel(10);
  let (delimiter_tx, delimiter_rx) = mpsc::channel(10);

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
    "delimiter".to_string(),
    Box::pin(ReceiverStream::new(delimiter_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, delimiter_tx, inputs)
}

#[tokio::test]
async fn test_string_split_node_creation() {
  let node = StringSplitNode::new("test_split".to_string());
  assert_eq!(node.name(), "test_split");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("delimiter"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_string_split_basic() {
  let node = StringSplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "a,b,c" split by "," = ["a", "b", "c"]
  let _ = in_tx
    .send(Arc::new("a,b,c".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = delimiter_tx
    .send(Arc::new(",".to_string()) as Arc<dyn Any + Send + Sync>)
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
  let vec = &results[0];
  assert_eq!(vec.len(), 3);
  assert_eq!(
    *vec[0].clone().downcast::<String>().unwrap(),
    "a".to_string()
  );
  assert_eq!(
    *vec[1].clone().downcast::<String>().unwrap(),
    "b".to_string()
  );
  assert_eq!(
    *vec[2].clone().downcast::<String>().unwrap(),
    "c".to_string()
  );
}

#[tokio::test]
async fn test_string_split_space_delimiter() {
  let node = StringSplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "hello world rust" split by " " = ["hello", "world", "rust"]
  let _ = in_tx
    .send(Arc::new("hello world rust".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = delimiter_tx
    .send(Arc::new(" ".to_string()) as Arc<dyn Any + Send + Sync>)
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
  let vec = &results[0];
  assert_eq!(vec.len(), 3);
  assert_eq!(
    *vec[0].clone().downcast::<String>().unwrap(),
    "hello".to_string()
  );
  assert_eq!(
    *vec[1].clone().downcast::<String>().unwrap(),
    "world".to_string()
  );
  assert_eq!(
    *vec[2].clone().downcast::<String>().unwrap(),
    "rust".to_string()
  );
}

#[tokio::test]
async fn test_string_split_empty_string() {
  let node = StringSplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "" split by "," = [""] (single empty string)
  let _ = in_tx
    .send(Arc::new("".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = delimiter_tx
    .send(Arc::new(",".to_string()) as Arc<dyn Any + Send + Sync>)
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
  let vec = &results[0];
  assert_eq!(vec.len(), 1);
  assert_eq!(
    *vec[0].clone().downcast::<String>().unwrap(),
    "".to_string()
  );
}

#[tokio::test]
async fn test_string_split_no_delimiter() {
  let node = StringSplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "hello" split by "," = ["hello"] (no delimiter found)
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = delimiter_tx
    .send(Arc::new(",".to_string()) as Arc<dyn Any + Send + Sync>)
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
  let vec = &results[0];
  assert_eq!(vec.len(), 1);
  assert_eq!(
    *vec[0].clone().downcast::<String>().unwrap(),
    "hello".to_string()
  );
}

#[tokio::test]
async fn test_string_split_multiple_delimiters() {
  let node = StringSplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "a,,b" split by "," = ["a", "", "b"] (empty string between delimiters)
  let _ = in_tx
    .send(Arc::new("a,,b".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = delimiter_tx
    .send(Arc::new(",".to_string()) as Arc<dyn Any + Send + Sync>)
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
  let vec = &results[0];
  assert_eq!(vec.len(), 3);
  assert_eq!(
    *vec[0].clone().downcast::<String>().unwrap(),
    "a".to_string()
  );
  assert_eq!(
    *vec[1].clone().downcast::<String>().unwrap(),
    "".to_string()
  );
  assert_eq!(
    *vec[2].clone().downcast::<String>().unwrap(),
    "b".to_string()
  );
}

#[tokio::test]
async fn test_string_split_non_string_input() {
  let node = StringSplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-string input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = delimiter_tx
    .send(Arc::new(",".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("input must be String"));
}

#[tokio::test]
async fn test_string_split_non_string_delimiter() {
  let node = StringSplitNode::new("test_split".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-string delimiter
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = delimiter_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("delimiter must be String"));
}
