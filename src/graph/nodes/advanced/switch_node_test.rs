//! Tests for SwitchNode

use crate::graph::node::InputStreams;
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
  let (data_tx, data_rx) = mpsc::channel(10);
  let (value_tx, value_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "value".to_string(),
    Box::pin(ReceiverStream::new(value_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, data_tx, value_tx, inputs)
}

#[tokio::test]
async fn test_switch_node_creation() {
  use crate::graph::nodes::advanced::SwitchNode;
  let node = SwitchNode::new("test_switch".to_string(), 3);
  assert_eq!(node.name(), "test_switch");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("value"));
  assert!(node.has_output_port("out_0"));
  assert!(node.has_output_port("out_1"));
  assert!(node.has_output_port("out_2"));
  assert!(node.has_output_port("default"));
  assert!(node.has_output_port("error"));
  assert_eq!(node.max_branches(), 3);
}

#[tokio::test]
async fn test_switch_basic_routing() {
  use crate::graph::nodes::advanced::{SwitchNode, switch_config};
  let node = SwitchNode::new("test_switch".to_string(), 3);

  let (config_tx, data_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create and send configuration
  let config = switch_config(|_data, value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let n = *arc_i32;
      if n == 0 {
        Ok(Some(0)) // Route to out_0
      } else if n == 1 {
        Ok(Some(1)) // Route to out_1
      } else {
        Ok(None) // Route to default
      }
    } else {
      Err("Expected i32".to_string())
    }
  });
  let _ = config_tx.send(config as Arc<dyn Any + Send + Sync>).await;

  // Send data and value
  let _ = data_tx
    .send(Arc::new("item1".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;

  drop(config_tx);
  drop(data_tx);
  drop(value_tx);

  let out_stream = outputs.remove("out_0").unwrap();
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
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(str_val) = results[0].clone().downcast::<String>() {
    assert_eq!(*str_val, "item1");
  } else {
    panic!("Result is not a String");
  }
}

#[tokio::test]
async fn test_switch_default_routing() {
  use crate::graph::nodes::advanced::{SwitchNode, switch_config};
  let node = SwitchNode::new("test_switch".to_string(), 3);

  let (config_tx, data_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create and send configuration
  let config = switch_config(|_data, value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let n = *arc_i32;
      if n == 0 {
        Ok(Some(0)) // Route to out_0
      } else if n == 1 {
        Ok(Some(1)) // Route to out_1
      } else {
        Ok(None) // Route to default
      }
    } else {
      Err("Expected i32".to_string())
    }
  });
  let _ = config_tx.send(config as Arc<dyn Any + Send + Sync>).await;

  // Send data and value (value 2 doesn't match any case)
  let _ = data_tx
    .send(Arc::new("item2".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;

  drop(config_tx);
  drop(data_tx);
  drop(value_tx);

  let default_stream = outputs.remove("default").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = default_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(str_val) = results[0].clone().downcast::<String>() {
    assert_eq!(*str_val, "item2");
  } else {
    panic!("Result is not a String");
  }
}

#[tokio::test]
async fn test_switch_multiple_cases() {
  use crate::graph::nodes::advanced::{SwitchNode, switch_config};
  let node = SwitchNode::new("test_switch".to_string(), 3);

  let (config_tx, data_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create and send configuration
  let config = switch_config(|_data, value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let n = *arc_i32;
      if n == 0 {
        Ok(Some(0)) // Route to out_0
      } else if n == 1 {
        Ok(Some(1)) // Route to out_1
      } else if n == 2 {
        Ok(Some(2)) // Route to out_2
      } else {
        Ok(None) // Route to default
      }
    } else {
      Err("Expected i32".to_string())
    }
  });
  let _ = config_tx.send(config as Arc<dyn Any + Send + Sync>).await;

  // Send multiple items with different values
  let _ = data_tx
    .send(Arc::new("item0".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;

  let _ = data_tx
    .send(Arc::new("item1".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;

  let _ = data_tx
    .send(Arc::new("item2".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;

  drop(config_tx);
  drop(data_tx);
  drop(value_tx);

  // Check out_0
  let out0_stream = outputs.remove("out_0").unwrap();
  let mut results0: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream0 = out0_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  loop {
    tokio::select! {
      result = stream0.next() => {
        if let Some(item) = result {
          results0.push(item);
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results0.len(), 1);
  if let Ok(str_val) = results0[0].clone().downcast::<String>() {
    assert_eq!(*str_val, "item0");
  } else {
    panic!("Result is not a String");
  }

  // Check out_1
  let out1_stream = outputs.remove("out_1").unwrap();
  let mut results1: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream1 = out1_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream1.next() => {
        if let Some(item) = result {
          results1.push(item);
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results1.len(), 1);
  if let Ok(str_val) = results1[0].clone().downcast::<String>() {
    assert_eq!(*str_val, "item1");
  } else {
    panic!("Result is not a String");
  }

  // Check out_2
  let out2_stream = outputs.remove("out_2").unwrap();
  let mut results2: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream2 = out2_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  loop {
    tokio::select! {
      result = stream2.next() => {
        if let Some(item) = result {
          results2.push(item);
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results2.len(), 1);
  if let Ok(str_val) = results2[0].clone().downcast::<String>() {
    assert_eq!(*str_val, "item2");
  } else {
    panic!("Result is not a String");
  }
}

#[tokio::test]
async fn test_switch_no_config() {
  use crate::graph::nodes::advanced::SwitchNode;
  let node = SwitchNode::new("test_switch".to_string(), 3);

  let (_config_tx, data_tx, value_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Send data and value without configuration
  let _ = data_tx
    .send(Arc::new("item1".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;

  drop(data_tx);
  drop(value_tx);

  let default_stream = outputs.remove("default").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  let mut stream = default_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);

  use tokio_stream::StreamExt;
  loop {
    tokio::select! {
      result = stream.next() => {
        if let Some(item) = result {
          results.push(item);
          break;
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  // Should route to default when no config
  assert_eq!(results.len(), 1);
  if let Ok(str_val) = results[0].clone().downcast::<String>() {
    assert_eq!(*str_val, "item1");
  } else {
    panic!("Result is not a String");
  }
}
