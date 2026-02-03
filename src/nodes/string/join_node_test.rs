//! Tests for StringJoinNode

use crate::node::{InputStreams, Node};
use crate::nodes::string::StringJoinNode;
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
async fn test_string_join_node_creation() {
  let node = StringJoinNode::new("test_join".to_string());
  assert_eq!(node.name(), "test_join");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("delimiter"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_string_join_basic() {
  let node = StringJoinNode::new("test_join".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: ["a", "b", "c"] join with "," = "a,b,c"
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("b".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("c".to_string()) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push(arc_str.clone());
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
  assert_eq!(*results[0], "a,b,c");
}

#[tokio::test]
async fn test_string_join_space_delimiter() {
  let node = StringJoinNode::new("test_join".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: ["hello", "world", "rust"] join with " " = "hello world rust"
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("world".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("rust".to_string()) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push(arc_str.clone());
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
  assert_eq!(*results[0], "hello world rust");
}

#[tokio::test]
async fn test_string_join_empty_array() {
  let node = StringJoinNode::new("test_join".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: [] join with "," = "" (empty string)
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push(arc_str.clone());
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
  assert_eq!(*results[0], "");
}

#[tokio::test]
async fn test_string_join_single_element() {
  let node = StringJoinNode::new("test_join".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: ["hello"] join with "," = "hello" (no delimiter)
  let vec: Vec<Arc<dyn Any + Send + Sync>> =
    vec![Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push(arc_str.clone());
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
  assert_eq!(*results[0], "hello");
}

#[tokio::test]
async fn test_string_join_empty_delimiter() {
  let node = StringJoinNode::new("test_join".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: ["a", "b", "c"] join with "" = "abc" (no delimiter)
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("b".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new("c".to_string()) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = delimiter_tx
    .send(Arc::new("".to_string()) as Arc<dyn Any + Send + Sync>)
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
          if let Ok(arc_str) = item.downcast::<String>() {
            results.push(arc_str.clone());
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
  assert_eq!(*results[0], "abc");
}

#[tokio::test]
async fn test_string_join_non_array_input() {
  let node = StringJoinNode::new("test_join".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-array input
  let _ = in_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("Vec"));
}

#[tokio::test]
async fn test_string_join_non_string_delimiter() {
  let node = StringJoinNode::new("test_join".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-string delimiter
  let vec: Vec<Arc<dyn Any + Send + Sync>> =
    vec![Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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

#[tokio::test]
async fn test_string_join_non_string_element() {
  let node = StringJoinNode::new("test_join".to_string());

  let (_config_tx, in_tx, delimiter_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send array with non-string element
  let vec: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new(42i32) as Arc<dyn Any + Send + Sync>, // Non-string element
    Arc::new("c".to_string()) as Arc<dyn Any + Send + Sync>,
  ];
  let _ = in_tx
    .send(Arc::new(vec) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("array element") && errors[0].contains("must be String"));
}
