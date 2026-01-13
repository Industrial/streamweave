//! # Match Node Test Suite
//!
//! Comprehensive test suite for the MatchNode implementation.

use crate::graph::node::{InputStreams, Node};
use crate::graph::nodes::match_node::{
  MatchConfig, MatchNode, match_config, match_exact_string, match_numeric_range, match_regex,
};
use regex::Regex;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Helper to create input streams from channels
fn create_input_streams() -> (
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  mpsc::Sender<Arc<dyn Any + Send + Sync>>,
  InputStreams,
) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (data_tx, data_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, data_tx, inputs)
}

#[tokio::test]
async fn test_match_node_creation() {
  let node = MatchNode::new("test_match".to_string(), 3);
  assert_eq!(node.name(), "test_match");
  assert_eq!(node.max_branches(), 3);
  assert!(!node.has_config());
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out_0"));
  assert!(node.has_output_port("out_1"));
  assert!(node.has_output_port("out_2"));
  assert!(node.has_output_port("default"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_match_node_numeric_range() {
  let node = MatchNode::new("test_match".to_string(), 2);

  // Create a config that routes based on number ranges
  // Negative -> out_0, 0-10 -> out_1, 10+ -> default
  let config: MatchConfig = match_numeric_range(|n: i32| async move {
    if n < 0 {
      Ok(Some(0))
    } else if n < 10 {
      Ok(Some(1))
    } else {
      Ok(None) // Route to default
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data: negative, small positive, large positive
  let _ = data_tx
    .send(Arc::new(-5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(15i32) as Arc<dyn Any + Send + Sync>)
    .await;

  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Check out_0 (negative numbers)
  let out_0_stream = outputs.remove("out_0").unwrap();
  let mut out_0_results = Vec::new();
  let mut stream = out_0_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            out_0_results.push(*arc_i32);
            if !out_0_results.is_empty() {
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

  assert_eq!(out_0_results.len(), 1);
  assert_eq!(out_0_results[0], -5);

  // Check out_1 (0-10 range)
  let out_1_stream = outputs.remove("out_1").unwrap();
  let mut out_1_results = Vec::new();
  let mut stream = out_1_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            out_1_results.push(*arc_i32);
            if !out_1_results.is_empty() {
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

  assert_eq!(out_1_results.len(), 1);
  assert_eq!(out_1_results[0], 5);

  // Check default (10+)
  let default_stream = outputs.remove("default").unwrap();
  let mut default_results = Vec::new();
  let mut stream = default_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            default_results.push(*arc_i32);
            if !default_results.is_empty() {
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

  assert_eq!(default_results.len(), 1);
  assert_eq!(default_results[0], 15);
}

#[tokio::test]
async fn test_match_node_regex() {
  let node = MatchNode::new("test_match".to_string(), 2);

  // Create a config that routes based on regex patterns
  // Email -> out_0, URL -> out_1, others -> default
  let patterns = vec![
    (
      Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap(),
      0,
    ),
    (Regex::new(r"^https?://").unwrap(), 1),
  ];
  let config: MatchConfig = match_regex(patterns);

  let (config_tx, data_tx, inputs) = create_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data: email, URL, other string
  let _ = data_tx
    .send(Arc::new("user@example.com".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new("https://example.com".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new("plain text".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Check out_0 (email)
  let out_0_stream = outputs.remove("out_0").unwrap();
  let mut out_0_results = Vec::new();
  let mut stream = out_0_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            out_0_results.push((*arc_str).clone());
            if !out_0_results.is_empty() {
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

  assert_eq!(out_0_results.len(), 1);
  assert_eq!(out_0_results[0], "user@example.com");

  // Check out_1 (URL)
  let out_1_stream = outputs.remove("out_1").unwrap();
  let mut out_1_results = Vec::new();
  let mut stream = out_1_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            out_1_results.push(arc_str.clone());
            if !out_1_results.is_empty() {
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

  assert_eq!(out_1_results.len(), 1);
  assert_eq!(&*out_1_results[0], "https://example.com");

  // Check default (other strings)
  let default_stream = outputs.remove("default").unwrap();
  let mut default_results = Vec::new();
  let mut stream = default_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            default_results.push(arc_str.clone());
            if !default_results.is_empty() {
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

  assert_eq!(default_results.len(), 1);
  assert_eq!(&*default_results[0], "plain text");
}

#[tokio::test]
async fn test_match_node_exact() {
  let node = MatchNode::new("test_match".to_string(), 2);

  // Create a config that routes based on exact string matching
  // "error" -> out_0, "warning" -> out_1, others -> default
  let patterns = vec![("error", 0), ("warning", 1)];
  let config: MatchConfig = match_exact_string(patterns);

  let (config_tx, data_tx, inputs) = create_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data: error, warning, info
  let _ = data_tx
    .send(Arc::new("error".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new("warning".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new("info".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Check out_0 (error)
  let out_0_stream = outputs.remove("out_0").unwrap();
  let mut out_0_results: Vec<String> = Vec::new();
  let mut stream = out_0_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            out_0_results.push((*arc_str).clone());
            if !out_0_results.is_empty() {
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

  assert_eq!(out_0_results.len(), 1);
  assert_eq!(&*out_0_results[0], "error");

  // Check out_1 (warning)
  let out_1_stream = outputs.remove("out_1").unwrap();
  let mut out_1_results = Vec::new();
  let mut stream = out_1_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            out_1_results.push(arc_str.clone());
            if !out_1_results.is_empty() {
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

  assert_eq!(out_1_results.len(), 1);
  assert_eq!(&*out_1_results[0], "warning");

  // Check default (info)
  let default_stream = outputs.remove("default").unwrap();
  let mut default_results = Vec::new();
  let mut stream = default_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            default_results.push(arc_str.clone());
            if !default_results.is_empty() {
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

  assert_eq!(default_results.len(), 1);
  assert_eq!(&*default_results[0], "info");
}

#[tokio::test]
async fn test_match_node_custom_pattern() {
  let node = MatchNode::new("test_match".to_string(), 3);

  // Create a custom config using match_config directly
  // Even numbers -> out_0, odd numbers -> out_1, zero -> out_2, negative -> default
  let config: MatchConfig = match_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let n = *arc_i32;
      if n == 0 {
        Ok(Some(2))
      } else if n % 2 == 0 {
        Ok(Some(0))
      } else if n > 0 {
        Ok(Some(1))
      } else {
        Ok(None) // Negative -> default
      }
    } else {
      Err("Expected i32".to_string())
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data: even, odd, zero, negative
  let _ = data_tx
    .send(Arc::new(4i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(-3i32) as Arc<dyn Any + Send + Sync>)
    .await;

  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Check out_0 (even)
  let out_0_stream = outputs.remove("out_0").unwrap();
  let mut out_0_results = Vec::new();
  let mut stream = out_0_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            out_0_results.push(*arc_i32);
            if !out_0_results.is_empty() {
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

  assert_eq!(out_0_results.len(), 1);
  assert_eq!(out_0_results[0], 4);

  // Check out_1 (odd)
  let out_1_stream = outputs.remove("out_1").unwrap();
  let mut out_1_results = Vec::new();
  let mut stream = out_1_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            out_1_results.push(*arc_i32);
            if !out_1_results.is_empty() {
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

  assert_eq!(out_1_results.len(), 1);
  assert_eq!(out_1_results[0], 5);

  // Check out_2 (zero)
  let out_2_stream = outputs.remove("out_2").unwrap();
  let mut out_2_results = Vec::new();
  let mut stream = out_2_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            out_2_results.push(*arc_i32);
            if !out_2_results.is_empty() {
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

  assert_eq!(out_2_results.len(), 1);
  assert_eq!(out_2_results[0], 0);

  // Check default (negative)
  let default_stream = outputs.remove("default").unwrap();
  let mut default_results = Vec::new();
  let mut stream = default_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            default_results.push(*arc_i32);
            if !default_results.is_empty() {
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

  assert_eq!(default_results.len(), 1);
  assert_eq!(default_results[0], -3);
}

#[tokio::test]
async fn test_match_node_error_handling() {
  let node = MatchNode::new("test_match".to_string(), 2);

  // Create a config that returns an error for certain inputs
  let config: MatchConfig = match_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let n = *arc_i32;
      if n < 0 {
        Err("Negative numbers not allowed".to_string())
      } else if n < 10 {
        Ok(Some(0))
      } else {
        Ok(Some(1))
      }
    } else {
      Err("Expected i32".to_string())
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data: valid, error case, valid
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(-5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(15i32) as Arc<dyn Any + Send + Sync>)
    .await;

  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Check error output
  let error_stream = outputs.remove("error").unwrap();
  let mut error_results: Vec<String> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            error_results.push((*arc_str).clone());
            if !error_results.is_empty() {
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

  assert_eq!(error_results.len(), 1);
  assert_eq!(&*error_results[0], "Negative numbers not allowed");

  // Check that valid items still route correctly
  let out_0_stream = outputs.remove("out_0").unwrap();
  let mut out_0_results = Vec::new();
  let mut stream = out_0_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            out_0_results.push(*arc_i32);
            if !out_0_results.is_empty() {
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

  assert_eq!(out_0_results.len(), 1);
  assert_eq!(out_0_results[0], 5);
}

#[tokio::test]
async fn test_match_node_no_config() {
  let node = MatchNode::new("test_match".to_string(), 2);

  let (_config_tx, data_tx, inputs) = create_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send data without configuration
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;

  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Check error output
  let error_stream = outputs.remove("error").unwrap();
  let mut error_results: Vec<String> = Vec::new();
  let mut stream = error_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_str) = item.downcast::<String>() {
            error_results.push((*arc_str).clone());
            if !error_results.is_empty() {
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

  assert_eq!(error_results.len(), 1);
  assert!(error_results[0].contains("No configuration set"));
}

#[tokio::test]
async fn test_match_node_branch_index_out_of_range() {
  let node = MatchNode::new("test_match".to_string(), 2); // Only out_0 and out_1

  // Create a config that returns branch index 5 (out of range)
  let config: MatchConfig = match_config(|value| async move {
    if let Ok(_arc_i32) = value.downcast::<i32>() {
      Ok(Some(5)) // Out of range - should route to default
    } else {
      Err("Expected i32".to_string())
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();

  // Execute the node
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send configuration
  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send data
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;

  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Check default (out of range branch should route to default)
  let default_stream = outputs.remove("default").unwrap();
  let mut default_results = Vec::new();
  let mut stream = default_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          if let Ok(arc_i32) = item.downcast::<i32>() {
            default_results.push(*arc_i32);
            if !default_results.is_empty() {
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

  assert_eq!(default_results.len(), 1);
  assert_eq!(default_results[0], 5);
}
