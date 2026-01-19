//! Tests for AggregateNode
#![allow(unused_imports, dead_code, unused, clippy::type_complexity)]

use crate::node::{InputStreams, Node, OutputStreams};
use crate::nodes::reduction::{
  AggregateConfig, AggregateConfigWrapper, AggregateNode, AggregatorFunction, aggregate_config,
};

use async_trait::async_trait;
use futures::StreamExt;
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
  let (aggregator_tx, aggregator_rx) = mpsc::channel(10);

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
    "aggregator".to_string(),
    Box::pin(ReceiverStream::new(aggregator_rx)) as crate::node::InputStream,
  );

  (config_tx, in_tx, aggregator_tx, inputs)
}

/// Simple sum aggregator for testing
struct SumAggregator {
  sum: i32,
}

#[async_trait]
impl AggregatorFunction for SumAggregator {
  async fn process_item(&mut self, value: Arc<dyn Any + Send + Sync>) -> Result<(), String> {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      self.sum += *arc_i32;
      Ok(())
    } else {
      Err("Expected i32".to_string())
    }
  }

  async fn finalize(&self) -> Result<Arc<dyn Any + Send + Sync>, String> {
    Ok(Arc::new(self.sum) as Arc<dyn Any + Send + Sync>)
  }
}

#[tokio::test]
async fn test_aggregate_node_creation() {
  let node = AggregateNode::new("test_aggregate".to_string());
  assert_eq!(node.name(), "test_aggregate");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_input_port("aggregator"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_aggregate_sum() {
  let node = AggregateNode::new("test_aggregate".to_string());

  let (_config_tx, in_tx, aggregator_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Create a sum aggregator
  let aggregator: AggregateConfig = aggregate_config(SumAggregator { sum: 0 });

  // Send aggregator
  let _ = aggregator_tx
    .send(Arc::new(AggregateConfigWrapper::new(aggregator)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: 1, 2, 3 → sum = 6
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx); // Close the input stream
  drop(aggregator_tx); // Close the aggregator stream

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
  if let Ok(sum) = results[0].clone().downcast::<i32>() {
    assert_eq!(*sum, 6i32);
  } else {
    panic!("Result is not an i32");
  }
}

#[tokio::test]
async fn test_aggregate_empty_stream() {
  let node = AggregateNode::new("test_aggregate".to_string());

  let (_config_tx, in_tx, aggregator_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Create a sum aggregator
  let aggregator: AggregateConfig = aggregate_config(SumAggregator { sum: 0 });

  // Send aggregator
  let _ = aggregator_tx
    .send(Arc::new(AggregateConfigWrapper::new(aggregator)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send no values: empty stream → result should be 0 (initial sum)
  drop(in_tx); // Close the input stream immediately
  drop(aggregator_tx); // Close the aggregator stream

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
  if let Ok(sum) = results[0].clone().downcast::<i32>() {
    assert_eq!(*sum, 0i32); // Should be the initial sum
  } else {
    panic!("Result is not an i32");
  }
}

/// Max aggregator for testing
struct MaxAggregator {
  max: Option<i32>,
}

#[async_trait]
impl AggregatorFunction for MaxAggregator {
  async fn process_item(&mut self, value: Arc<dyn Any + Send + Sync>) -> Result<(), String> {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      match self.max {
        None => self.max = Some(*arc_i32),
        Some(current_max) => {
          if *arc_i32 > current_max {
            self.max = Some(*arc_i32);
          }
        }
      }
      Ok(())
    } else {
      Err("Expected i32".to_string())
    }
  }

  async fn finalize(&self) -> Result<Arc<dyn Any + Send + Sync>, String> {
    match self.max {
      Some(max_val) => Ok(Arc::new(max_val) as Arc<dyn Any + Send + Sync>),
      None => Err("No values processed".to_string()),
    }
  }
}

#[tokio::test]
async fn test_aggregate_max() {
  let node = AggregateNode::new("test_aggregate".to_string());

  let (_config_tx, in_tx, aggregator_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Create a max aggregator
  let aggregator: AggregateConfig = aggregate_config(MaxAggregator { max: None });

  // Send aggregator
  let _ = aggregator_tx
    .send(Arc::new(AggregateConfigWrapper::new(aggregator)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values: 1, 5, 3, 7, 2 → max = 7
  let _ = in_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(7i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;
  drop(in_tx);
  drop(aggregator_tx);

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
  if let Ok(max) = results[0].clone().downcast::<i32>() {
    assert_eq!(*max, 7i32);
  } else {
    panic!("Result is not an i32");
  }
}

/// Error aggregator for testing error handling
struct ErrorAggregator {
  count: i32,
}

#[async_trait]
impl AggregatorFunction for ErrorAggregator {
  async fn process_item(&mut self, value: Arc<dyn Any + Send + Sync>) -> Result<(), String> {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      if *arc_i32 < 0 {
        Err("Negative values not allowed".to_string())
      } else {
        self.count += 1;
        Ok(())
      }
    } else {
      Err("Expected i32".to_string())
    }
  }

  async fn finalize(&self) -> Result<Arc<dyn Any + Send + Sync>, String> {
    Ok(Arc::new(self.count) as Arc<dyn Any + Send + Sync>)
  }
}

#[tokio::test]
async fn test_aggregate_error_handling() {
  let node = AggregateNode::new("test_aggregate".to_string());

  let (_config_tx, in_tx, aggregator_tx, inputs) = create_input_streams();
  let mut outputs: OutputStreams = match node.execute(inputs).await {
    Ok(outputs) => outputs,
    Err(e) => panic!("Node execution failed: {}", e),
  };

  // Create an error aggregator
  let aggregator: AggregateConfig = aggregate_config(ErrorAggregator { count: 0 });

  // Send aggregator
  let _ = aggregator_tx
    .send(Arc::new(AggregateConfigWrapper::new(aggregator)) as Arc<dyn Any + Send + Sync>)
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
  drop(aggregator_tx);

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

  // Check that valid items are still processed
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
  if let Ok(count) = results[0].clone().downcast::<i32>() {
    // Should count 2 valid items (1 and 3, not -2)
    assert_eq!(*count, 2i32);
  } else {
    panic!("Result is not an i32");
  }
}
