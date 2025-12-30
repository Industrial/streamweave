//! Router trait tests
//!
//! This module provides integration tests for router traits (InputRouter, OutputRouter).

use futures::{StreamExt, stream};
use std::pin::Pin;
use streamweave_graph::{InputRouter, OutputRouter, RouterError};

// Test helper: Simple stateless merge router
struct TestMergeRouter {
  expected_ports: Vec<usize>,
}

#[async_trait::async_trait]
impl<I> InputRouter<I> for TestMergeRouter
where
  I: Send + Sync + 'static,
{
  async fn route_streams(
    &mut self,
    streams: Vec<(usize, Pin<Box<dyn futures::Stream<Item = I> + Send>>)>,
  ) -> Pin<Box<dyn futures::Stream<Item = I> + Send>> {
    Box::pin(futures::stream::select_all(
      streams.into_iter().map(|(_, stream)| stream),
    ))
  }

  fn expected_ports(&self) -> Vec<usize> {
    self.expected_ports.clone()
  }
}

#[tokio::test]
async fn test_input_router_trait() {
  let mut router = TestMergeRouter {
    expected_ports: vec![0, 1],
  };

  // Create test streams
  let stream1: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));
  let stream2: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![4, 5, 6]));

  let streams = vec![(0, stream1), (1, stream2)];
  let mut merged = router.route_streams(streams).await;

  // Collect results
  let mut results = Vec::new();
  while let Some(item) = merged.next().await {
    results.push(item);
  }

  // Should have all items (order may vary due to select_all)
  assert_eq!(results.len(), 6);
  assert!(results.contains(&1));
  assert!(results.contains(&2));
  assert!(results.contains(&3));
  assert!(results.contains(&4));
  assert!(results.contains(&5));
  assert!(results.contains(&6));
}

#[test]
fn test_expected_ports() {
  let router = TestMergeRouter {
    expected_ports: vec![0, 1, 2],
  };

  let ports = <TestMergeRouter as InputRouter<i32>>::expected_ports(&router);
  assert_eq!(ports, vec![0, 1, 2]);
}

#[test]
fn test_router_error_display() {
  let error = RouterError::InvalidPort {
    port: 5,
    expected: vec![0, 1, 2],
  };
  assert_eq!(
    error.to_string(),
    "Invalid port index: 5 (expected one of: [0, 1, 2])"
  );

  let error = RouterError::StreamError {
    message: "Connection lost".to_string(),
  };
  assert_eq!(error.to_string(), "Stream error: Connection lost");

  let error = RouterError::ConfigurationError {
    message: "Invalid strategy".to_string(),
  };
  assert_eq!(error.to_string(), "Configuration error: Invalid strategy");
}

// Test helper: Simple pass-through router
struct TestPassThroughRouter {
  output_ports: Vec<usize>,
}

#[async_trait::async_trait]
impl<O> OutputRouter<O> for TestPassThroughRouter
where
  O: Send + Sync + 'static,
{
  async fn route_stream(
    &mut self,
    stream: Pin<Box<dyn futures::Stream<Item = O> + Send>>,
  ) -> Vec<(usize, Pin<Box<dyn futures::Stream<Item = O> + Send>>)> {
    // Minimal implementation: pass stream to first port only
    if let Some(&first_port) = self.output_ports.first() {
      vec![(first_port, stream)]
    } else {
      vec![]
    }
  }

  fn output_ports(&self) -> Vec<usize> {
    self.output_ports.clone()
  }
}

#[tokio::test]
async fn test_output_router_trait() {
  let mut router = TestPassThroughRouter {
    output_ports: vec![0, 1, 2],
  };

  // Create test stream
  let input_stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> =
    Box::pin(stream::iter(vec![1, 2, 3]));

  let output_streams = router.route_stream(input_stream).await;

  // Should have at least one stream
  assert!(!output_streams.is_empty());

  // Verify port index is valid
  if let Some((port, _)) = output_streams.first() {
    assert!(<TestPassThroughRouter as OutputRouter<i32>>::output_ports(&router).contains(port));
  }
}

#[test]
fn test_output_ports() {
  let router = TestPassThroughRouter {
    output_ports: vec![0, 1, 2],
  };

  let ports = <TestPassThroughRouter as OutputRouter<i32>>::output_ports(&router);
  assert_eq!(ports, vec![0, 1, 2]);
}

#[test]
fn test_router_error_clone() {
  let error1 = RouterError::InvalidPort {
    port: 5,
    expected: vec![0, 1, 2],
  };
  let error2 = error1.clone();

  assert_eq!(error1, error2);
}

#[test]
fn test_router_error_partial_eq() {
  let error1 = RouterError::StreamError {
    message: "test".to_string(),
  };
  let error2 = RouterError::StreamError {
    message: "test".to_string(),
  };
  let error3 = RouterError::StreamError {
    message: "different".to_string(),
  };

  assert_eq!(error1, error2);
  assert_ne!(error1, error3);
}

#[test]
fn test_router_error_debug() {
  let error = RouterError::ConfigurationError {
    message: "test".to_string(),
  };
  let debug_str = format!("{:?}", error);
  assert!(!debug_str.is_empty());
}

#[test]
fn test_router_error_error_trait() {
  use std::error::Error;

  let error = RouterError::StreamError {
    message: "test".to_string(),
  };

  // Verify it implements Error trait
  let _error_ref: &dyn Error = &error;
  assert_eq!(error.to_string(), "Stream error: test");
}
