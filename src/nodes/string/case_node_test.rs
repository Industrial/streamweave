//! Tests for StringCaseNode

use crate::node::{InputStreams, Node};
use crate::nodes::string::StringCaseNode;
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
  let (case_type_tx, case_type_rx) = mpsc::channel(10);

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
    "case_type".to_string(),
    Box::pin(ReceiverStream::new(case_type_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, case_type_tx, inputs)
}

#[tokio::test]
async fn test_string_case_node_creation() {
  let node = StringCaseNode::new("test_case".to_string());
  assert_eq!(node.name(), "test_case");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("case_type"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_string_case_upper() {
  let node = StringCaseNode::new("test_case".to_string());

  let (_config_tx, in_tx, case_type_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" with "upper" = "HELLO WORLD"
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = case_type_tx
    .send(Arc::new("upper".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(*results[0], "HELLO WORLD");
}

#[tokio::test]
async fn test_string_case_uppercase() {
  let node = StringCaseNode::new("test_case".to_string());

  let (_config_tx, in_tx, case_type_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" with "uppercase" = "HELLO WORLD"
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = case_type_tx
    .send(Arc::new("uppercase".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(*results[0], "HELLO WORLD");
}

#[tokio::test]
async fn test_string_case_lower() {
  let node = StringCaseNode::new("test_case".to_string());

  let (_config_tx, in_tx, case_type_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" with "lower" = "hello world"
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = case_type_tx
    .send(Arc::new("lower".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(*results[0], "hello world");
}

#[tokio::test]
async fn test_string_case_lowercase() {
  let node = StringCaseNode::new("test_case".to_string());

  let (_config_tx, in_tx, case_type_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "Hello World" with "lowercase" = "hello world"
  let _ = in_tx
    .send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = case_type_tx
    .send(Arc::new("lowercase".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(*results[0], "hello world");
}

#[tokio::test]
async fn test_string_case_title() {
  let node = StringCaseNode::new("test_case".to_string());

  let (_config_tx, in_tx, case_type_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "hello world" with "title" = "Hello World"
  let _ = in_tx
    .send(Arc::new("hello world".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = case_type_tx
    .send(Arc::new("title".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(*results[0], "Hello World");
}

#[tokio::test]
async fn test_string_case_titlecase() {
  let node = StringCaseNode::new("test_case".to_string());

  let (_config_tx, in_tx, case_type_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "HELLO WORLD" with "titlecase" = "Hello World"
  let _ = in_tx
    .send(Arc::new("HELLO WORLD".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = case_type_tx
    .send(Arc::new("titlecase".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert_eq!(*results[0], "Hello World");
}

#[tokio::test]
async fn test_string_case_empty_string() {
  let node = StringCaseNode::new("test_case".to_string());

  let (_config_tx, in_tx, case_type_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send values: "" with "upper" = ""
  let _ = in_tx
    .send(Arc::new("".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = case_type_tx
    .send(Arc::new("upper".to_string()) as Arc<dyn Any + Send + Sync>)
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
async fn test_string_case_invalid_case_type() {
  let node = StringCaseNode::new("test_case".to_string());

  let (_config_tx, in_tx, case_type_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send invalid case type: "invalid"
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = case_type_tx
    .send(Arc::new("invalid".to_string()) as Arc<dyn Any + Send + Sync>)
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
  assert!(errors[0].contains("Unsupported case type"));
}

#[tokio::test]
async fn test_string_case_non_string_input() {
  let node = StringCaseNode::new("test_case".to_string());

  let (_config_tx, in_tx, case_type_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-string input
  let _ = in_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = case_type_tx
    .send(Arc::new("upper".to_string()) as Arc<dyn Any + Send + Sync>)
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
async fn test_string_case_non_string_case_type() {
  let node = StringCaseNode::new("test_case".to_string());

  let (_config_tx, in_tx, case_type_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send non-string case type
  let _ = in_tx
    .send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = case_type_tx
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
  assert!(errors[0].contains("case type must be String"));
}
