//! # Node Trait Test Suite - Stream-Based
//!
//! Comprehensive test suite for the [`Node`] trait with stream-based architecture,
//! including name management, port queries, and stream execution.
//!
//! ## Test Coverage
//!
//! This test suite covers:
//!
//! - **Name Management**: Getting and setting node names
//! - **Port Queries**: Querying input and output port names
//! - **Port Validation**: Checking if ports exist by name
//! - **Stream Execution**: Executing nodes with input streams and producing output streams

use crate::graph::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};

// ============================================================================
// Mock Node Implementations for Testing
// ============================================================================

/// Mock producer node (0 inputs, 1 output)
struct MockProducerNode {
  name: String,
  output_port_names: Vec<String>,
}

impl MockProducerNode {
  fn new(name: String) -> Self {
    Self {
      name,
      output_port_names: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for MockProducerNode {
  fn name(&self) -> &str {
    &self.name
  }

  fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }

  fn input_port_names(&self) -> &[String] {
    &[]
  }

  fn output_port_names(&self) -> &[String] {
    &self.output_port_names
  }

  fn has_input_port(&self, _name: &str) -> bool {
    false
  }

  fn has_output_port(&self, name: &str) -> bool {
    self.output_port_names.contains(&name.to_string())
  }

  fn execute(
    &self,
    _inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    Box::pin(async move {
      let (tx, rx) = mpsc::channel(10);

      // Send some test data
      tokio::spawn(async move {
        for i in 0..3 {
          let _ = tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
        }
      });

      let stream: OutputStreams = {
        let mut map = HashMap::new();
        map.insert(
          "out".to_string(),
          Box::pin(ReceiverStream::new(rx))
            as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
        );
        map
      };

      Ok(stream)
    })
  }
}

/// Mock transform node (1 input, 1 output)
struct MockTransformNode {
  name: String,
  input_port_names: Vec<String>,
  output_port_names: Vec<String>,
}

impl MockTransformNode {
  fn new(name: String) -> Self {
    Self {
      name,
      input_port_names: vec!["in".to_string()],
      output_port_names: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for MockTransformNode {
  fn name(&self) -> &str {
    &self.name
  }

  fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }

  fn input_port_names(&self) -> &[String] {
    &self.input_port_names
  }

  fn output_port_names(&self) -> &[String] {
    &self.output_port_names
  }

  fn has_input_port(&self, name: &str) -> bool {
    self.input_port_names.contains(&name.to_string())
  }

  fn has_output_port(&self, name: &str) -> bool {
    self.output_port_names.contains(&name.to_string())
  }

  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    Box::pin(async move {
      let input_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

      let output_stream: OutputStreams = {
        let mut map = HashMap::new();
        map.insert(
          "out".to_string(),
          Box::pin(async_stream::stream! {
            let mut input = input_stream;
            while let Some(item) = input.next().await {
              // Double the value if it's an i32
              if let Ok(arc_i32) = item.clone().downcast::<i32>() {
                yield Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>;
              } else {
                yield item;
              }
            }
          }) as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
        );
        map
      };

      Ok(output_stream)
    })
  }
}

/// Mock sink node (1 input, 0 outputs)
struct MockSinkNode {
  name: String,
  input_port_names: Vec<String>,
  received: Arc<tokio::sync::Mutex<Vec<Arc<dyn Any + Send + Sync>>>>,
}

impl MockSinkNode {
  fn new(name: String) -> Self {
    Self {
      name,
      input_port_names: vec!["in".to_string()],
      received: Arc::new(tokio::sync::Mutex::new(Vec::new())),
    }
  }

  #[allow(dead_code)]
  async fn get_received(&self) -> Vec<Arc<dyn Any + Send + Sync>> {
    self.received.lock().await.clone()
  }
}

#[async_trait]
impl Node for MockSinkNode {
  fn name(&self) -> &str {
    &self.name
  }

  fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }

  fn input_port_names(&self) -> &[String] {
    &self.input_port_names
  }

  fn output_port_names(&self) -> &[String] {
    &[]
  }

  fn has_input_port(&self, name: &str) -> bool {
    self.input_port_names.contains(&name.to_string())
  }

  fn has_output_port(&self, _name: &str) -> bool {
    false
  }

  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    let received = Arc::clone(&self.received);
    Box::pin(async move {
      let input_stream = inputs.remove("in").ok_or("Missing 'in' input")?;

      // Consume the stream
      tokio::spawn(async move {
        let mut input = input_stream;
        while let Some(item) = input.next().await {
          received.lock().await.push(item);
        }
      });

      Ok(HashMap::new())
    })
  }
}

// ============================================================================
// Name Management Tests
// ============================================================================

#[tokio::test]
async fn test_node_name_get() {
  let node = MockProducerNode::new("test_node".to_string());
  assert_eq!(node.name(), "test_node");
}

#[tokio::test]
async fn test_node_name_set() {
  let mut node = MockProducerNode::new("old_name".to_string());
  node.set_name("new_name");
  assert_eq!(node.name(), "new_name");
}

// ============================================================================
// Port Query Tests
// ============================================================================

#[tokio::test]
async fn test_producer_node_ports() {
  let node = MockProducerNode::new("producer".to_string());
  assert_eq!(node.input_port_names().len(), 0);
  assert_eq!(node.output_port_names().len(), 1);
  assert_eq!(node.output_port_names()[0], "out");
}

#[tokio::test]
async fn test_transform_node_ports() {
  let node = MockTransformNode::new("transform".to_string());
  assert_eq!(node.input_port_names().len(), 1);
  assert_eq!(node.input_port_names()[0], "in");
  assert_eq!(node.output_port_names().len(), 1);
  assert_eq!(node.output_port_names()[0], "out");
}

#[tokio::test]
async fn test_sink_node_ports() {
  let node = MockSinkNode::new("sink".to_string());
  assert_eq!(node.input_port_names().len(), 1);
  assert_eq!(node.input_port_names()[0], "in");
  assert_eq!(node.output_port_names().len(), 0);
}

// ============================================================================
// Port Validation Tests
// ============================================================================

#[tokio::test]
async fn test_has_input_port() {
  let node = MockTransformNode::new("transform".to_string());
  assert!(node.has_input_port("in"));
  assert!(!node.has_input_port("out"));
  assert!(!node.has_input_port("invalid"));
}

#[tokio::test]
async fn test_has_output_port() {
  let node = MockTransformNode::new("transform".to_string());
  assert!(node.has_output_port("out"));
  assert!(!node.has_output_port("in"));
  assert!(!node.has_output_port("invalid"));
}

// ============================================================================
// Stream Execution Tests
// ============================================================================

#[tokio::test]
async fn test_producer_node_execution() {
  let node = MockProducerNode::new("producer".to_string());
  let inputs = HashMap::new();
  let mut outputs = node.execute(inputs).await.unwrap();

  assert_eq!(outputs.len(), 1);
  assert!(outputs.contains_key("out"));

  // Consume the output stream
  let mut stream = outputs.remove("out").unwrap();
  let mut count = 0;
  while let Some(_item) = stream.next().await {
    count += 1;
  }
  assert_eq!(count, 3);
}

#[tokio::test]
async fn test_transform_node_execution() {
  let node = MockTransformNode::new("transform".to_string());

  // Create input stream
  let (tx, rx) = mpsc::channel(10);
  let mut inputs = HashMap::new();
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(rx))
      as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );

  // Send test data
  tokio::spawn(async move {
    for i in 1..=3 {
      let _ = tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
    }
  });

  // Execute node
  let mut outputs = node.execute(inputs).await.unwrap();
  assert_eq!(outputs.len(), 1);
  assert!(outputs.contains_key("out"));

  // Consume output stream and verify transformation
  let mut stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    if let Ok(arc_i32) = item.downcast::<i32>() {
      results.push(*arc_i32);
    }
  }

  assert_eq!(results, vec![2, 4, 6]);
}

#[tokio::test]
async fn test_sink_node_execution() {
  let node = MockSinkNode::new("sink".to_string());

  // Create input stream
  let (tx, rx) = mpsc::channel(10);
  let mut inputs = HashMap::new();
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(rx))
      as Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );

  // Send test data
  tokio::spawn(async move {
    for i in 1..=3 {
      let _ = tx.send(Arc::new(i) as Arc<dyn Any + Send + Sync>).await;
    }
  });

  // Execute node
  let outputs = node.execute(inputs).await.unwrap();
  assert_eq!(outputs.len(), 0);

  // Wait a bit for processing
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  // Check received data
  let received = node.get_received().await;
  assert_eq!(received.len(), 3);
}

#[tokio::test]
async fn test_node_execution_missing_input() {
  let node = MockTransformNode::new("transform".to_string());
  let inputs = HashMap::new(); // Missing "in" input

  let result = node.execute(inputs).await;
  assert!(result.is_err());
  // Just verify it's an error - we can't format the error due to Debug trait bounds
  // The error type contains streams which don't implement Debug
}
