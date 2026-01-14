//! Tests for GroupByNode

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
  let (in_tx, in_rx) = mpsc::channel(10);
  let (key_function_tx, key_function_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(in_rx)) as crate::graph::node::InputStream,
  );
  inputs.insert(
    "key_function".to_string(),
    Box::pin(ReceiverStream::new(key_function_rx)) as crate::graph::node::InputStream,
  );

  (config_tx, in_tx, key_function_tx, inputs)
}

#[tokio::test]
async fn test_group_by_node_creation() {
  let node = GroupByNode::new("test_group_by".to_string());
  assert_eq!(node.name(), "test_group_by");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("key_function"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_group_by_simple() {
  let node = GroupByNode::new("test_group_by".to_string());

  let (_config_tx, in_tx, key_function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a key function that extracts the value itself as a string
  let key_function: GroupByConfig = group_by_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(arc_i32.to_string())
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send key function
  let _ = key_function_tx
    .send(Arc::new(GroupByConfigWrapper::new(key_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: 1, 2, 1, 3, 2 → groups: {"1": [1, 1], "2": [2, 2], "3": [3]}
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx); // Close the input stream
  drop(key_function_tx); // Close the key function stream

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
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(grouped) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    // Check that we have 3 groups
    assert_eq!(grouped.len(), 3);

    // Check group "1" has 2 items
    if let Some(items_arc) = grouped.get("1") {
      if let Ok(items) = items_arc
        .clone()
        .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
      {
        assert_eq!(items.len(), 2);
        if let (Ok(item1), Ok(item2)) = (
          items[0].clone().downcast::<i32>(),
          items[1].clone().downcast::<i32>(),
        ) {
          assert_eq!(*item1, 1i32);
          assert_eq!(*item2, 1i32);
        } else {
          panic!("Items in group '1' are not i32");
        }
      } else {
        panic!("Group '1' is not a Vec");
      }
    } else {
      panic!("Group '1' not found");
    }

    // Check group "2" has 2 items
    if let Some(items_arc) = grouped.get("2") {
      if let Ok(items) = items_arc
        .clone()
        .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
      {
        assert_eq!(items.len(), 2);
      } else {
        panic!("Group '2' is not a Vec");
      }
    } else {
      panic!("Group '2' not found");
    }

    // Check group "3" has 1 item
    if let Some(items_arc) = grouped.get("3") {
      if let Ok(items) = items_arc
        .clone()
        .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
      {
        assert_eq!(items.len(), 1);
      } else {
        panic!("Group '3' is not a Vec");
      }
    } else {
      panic!("Group '3' not found");
    }
  } else {
    panic!("Result is not a HashMap<String, Arc<dyn Any + Send + Sync>>");
  }
}

#[tokio::test]
async fn test_group_by_empty_stream() {
  let node = GroupByNode::new("test_group_by".to_string());

  let (_config_tx, in_tx, key_function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a key function
  let key_function: GroupByConfig = group_by_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(arc_i32.to_string())
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send key function
  let _ = key_function_tx
    .send(Arc::new(GroupByConfigWrapper::new(key_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send no values: empty stream → result should be empty HashMap
  drop(in_tx); // Close the input stream immediately
  drop(key_function_tx); // Close the key function stream

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
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(grouped) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    // Empty stream should result in empty HashMap
    assert_eq!(grouped.len(), 0);
  } else {
    panic!("Result is not a HashMap<String, Arc<dyn Any + Send + Sync>>");
  }
}

#[tokio::test]
async fn test_group_by_key_function_error() {
  let node = GroupByNode::new("test_group_by".to_string());

  let (_config_tx, in_tx, key_function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a key function that returns an error for negative values
  let key_function: GroupByConfig = group_by_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      if *arc_i32 < 0 {
        Err("Negative values not allowed".to_string())
      } else {
        Ok(arc_i32.to_string())
      }
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Send key function
  let _ = key_function_tx
    .send(Arc::new(GroupByConfigWrapper::new(key_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: 1, -2, 3 → should error on -2
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(-2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(key_function_tx);

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
  assert_eq!(&*errors[0], "Negative values not allowed");

  // Check that valid items are still grouped
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
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(grouped) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    // Should have groups for "1" and "3" (valid values)
    assert!(grouped.contains_key("1"));
    assert!(grouped.contains_key("3"));
    assert!(!grouped.contains_key("-2")); // Error value not grouped
  } else {
    panic!("Result is not a HashMap<String, Arc<dyn Any + Send + Sync>>");
  }
}

#[tokio::test]
async fn test_group_by_string_values() {
  let node = GroupByNode::new("test_group_by".to_string());

  let (_config_tx, in_tx, key_function_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  // Create a key function that extracts first character
  let key_function: GroupByConfig = group_by_config(|value| async move {
    if let Ok(arc_str) = value.downcast::<String>() {
      if let Some(first_char) = arc_str.chars().next() {
        Ok(first_char.to_string())
      } else {
        Err("Empty string".to_string())
      }
    } else {
      Err("Expected String".to_string())
    }
  });

  // Send key function
  let _ = key_function_tx
    .send(Arc::new(GroupByConfigWrapper::new(key_function)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: "apple", "banana", "apricot" → groups: {"a": ["apple", "apricot"], "b": ["banana"]}
  let _ = in_tx
    .send(Arc::new("apple".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new("banana".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new("apricot".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(key_function_tx);

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
        } else {
          break;
        }
      }
      _ = &mut timeout => break,
    }
  }

  assert_eq!(results.len(), 1);
  if let Ok(grouped) = results[0]
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    assert_eq!(grouped.len(), 2);
    assert!(grouped.contains_key("a"));
    assert!(grouped.contains_key("b"));
  } else {
    panic!("Result is not a HashMap<String, Arc<dyn Any + Send + Sync>>");
  }
}
