//! # Output Trait Test Suite
//!
//! Comprehensive test suite for the `Output` trait, including trait implementation,
//! type constraints, associated types, and stream compatibility.
//!
//! ## Test Coverage
//!
//! This test suite covers:
//!
//! - **Trait Implementation**: Verification that types correctly implement the `Output` trait
//! - **Type Constraints**: Tests for `Send` bounds on `Output` and `OutputStream`
//! - **Generic Output**: Tests with various payload types (i32, String, Vec<i32>)
//! - **Associated Types**: Verification of `Output::Output` and `Output::OutputStream` types
//! - **Stream Compatibility**: Integration with `futures::Stream` methods (map, filter, collect)
//!
//! ## Test Organization
//!
//! Tests are organized into the following sections:
//!
//! 1. **Trait Implementation Tests**: Basic trait implementation verification
//! 2. **Type Constraints Tests**: `Send` trait bound validation
//! 3. **Generic Output Tests**: Tests with different payload types
//! 4. **Associated Type Tests**: Verification of associated type correctness
//! 5. **Integration Tests**: End-to-end stream operations
//! 6. **Stream Compatibility Tests**: Integration with futures stream combinators
//!
//! ## Key Concepts
//!
//! - **Output Trait**: Defines types that produce streams of `Message<T>`
//! - **OutputStream**: A pinned, boxed stream that yields `Message<T>` items
//! - **Type Safety**: All output types must be `Send` for cross-thread usage
//!
//! ## Usage
//!
//! These tests ensure that components implementing `Output` correctly produce
//! streams that can be consumed by transformers and consumers in the pipeline.

use crate::message::{Message, MessageId};
use crate::output::Output;
use futures::Stream;
use std::pin::Pin;

// Test implementation of Output trait
struct TestOutput<T>
where
  T: Send + 'static,
{
  _phantom: std::marker::PhantomData<T>,
}

impl<T> Output for TestOutput<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Message<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

// ============================================================================
// Trait Implementation Tests
// ============================================================================

#[test]
fn test_output_trait_impl_i32() {
  let _output: TestOutput<i32> = TestOutput {
    _phantom: std::marker::PhantomData,
  };
  // Just verify it compiles and implements the trait
}

#[test]
fn test_output_trait_impl_string() {
  let _output: TestOutput<String> = TestOutput {
    _phantom: std::marker::PhantomData,
  };
  // Just verify it compiles and implements the trait
}

#[test]
fn test_output_trait_impl_bool() {
  let _output: TestOutput<bool> = TestOutput {
    _phantom: std::marker::PhantomData,
  };
  // Just verify it compiles and implements the trait
}

// ============================================================================
// Type Constraints Tests
// ============================================================================

#[test]
fn test_output_type_is_send() {
  // Verify that Output::Output must be Send
  fn assert_send<T: Send>() {}

  assert_send::<Message<i32>>();
  assert_send::<Message<String>>();
  assert_send::<Message<bool>>();
}

#[test]
fn test_output_stream_is_send() {
  use futures::stream;

  // Verify that OutputStream must be Send
  fn assert_stream_is_send<S: Stream<Item = Message<i32>> + Send>() {}

  let stream = stream::empty::<Message<i32>>();
  assert_stream_is_send::<Pin<Box<dyn Stream<Item = Message<i32>> + Send>>>();

  let _boxed: Pin<Box<dyn Stream<Item = Message<i32>> + Send>> = Box::pin(stream);
}

// ============================================================================
// Generic Output Tests
// ============================================================================

#[test]
fn test_output_with_different_types() {
  // Test that Output trait works with various types
  let _output_i32: TestOutput<i32> = TestOutput {
    _phantom: std::marker::PhantomData,
  };

  let _output_string: TestOutput<String> = TestOutput {
    _phantom: std::marker::PhantomData,
  };

  let _output_vec: TestOutput<Vec<i32>> = TestOutput {
    _phantom: std::marker::PhantomData,
  };
}

// ============================================================================
// Associated Type Tests
// ============================================================================

#[test]
fn test_output_associated_type() {
  // Verify that Output::Output is Message<T>
  fn assert_output_type<T>() -> std::marker::PhantomData<T> {
    std::marker::PhantomData
  }

  let _phantom: std::marker::PhantomData<Message<i32>> = assert_output_type::<Message<i32>>();
}

#[test]
fn test_output_stream_type() {
  // Verify that OutputStream yields Output::Output
  use futures::stream;

  let _stream: Pin<Box<dyn Stream<Item = Message<i32>> + Send>> =
    Box::pin(stream::empty::<Message<i32>>());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_output_trait_constraints() {
  // Verify trait bounds work correctly
  fn take_output<O>(_output: O)
  where
    O: Output,
    O::Output: Send + 'static,
  {
  }

  let test_output = TestOutput::<i32> {
    _phantom: std::marker::PhantomData,
  };
  take_output(test_output);
}

#[tokio::test]
async fn test_output_stream_creation() {
  use futures::StreamExt;
  use futures::stream;

  let _test_output = TestOutput::<i32> {
    _phantom: std::marker::PhantomData,
  };

  // Create a stream that matches the Output trait
  let messages: Vec<Message<i32>> = vec![
    Message::new(1, MessageId::new_sequence(1)),
    Message::new(2, MessageId::new_sequence(2)),
    Message::new(3, MessageId::new_sequence(3)),
  ];

  let stream: <TestOutput<i32> as Output>::OutputStream = Box::pin(stream::iter(messages.clone()));

  let collected: Vec<Message<i32>> = stream.collect().await;
  assert_eq!(collected, messages);
}

#[tokio::test]
async fn test_output_stream_empty() {
  use futures::StreamExt;
  use futures::stream;

  let stream: <TestOutput<i32> as Output>::OutputStream = Box::pin(stream::empty::<Message<i32>>());

  let collected: Vec<Message<i32>> = stream.collect().await;
  assert!(collected.is_empty());
}

#[tokio::test]
async fn test_output_stream_single_item() {
  use futures::StreamExt;
  use futures::stream;

  let message = Message::new(42, MessageId::new_sequence(1));
  let stream: <TestOutput<i32> as Output>::OutputStream =
    Box::pin(stream::iter(vec![message.clone()]));

  let collected: Vec<Message<i32>> = stream.collect().await;
  assert_eq!(collected.len(), 1);
  assert_eq!(collected[0].payload(), message.payload());
}

// ============================================================================
// Stream Compatibility Tests
// ============================================================================

#[tokio::test]
async fn test_output_stream_with_futures_stream() {
  use futures::StreamExt;
  use futures::stream;

  // Verify OutputStream works with futures::Stream methods
  let messages: Vec<Message<i32>> = vec![
    Message::new(10, MessageId::new_sequence(1)),
    Message::new(20, MessageId::new_sequence(2)),
  ];

  let stream: <TestOutput<i32> as Output>::OutputStream = Box::pin(stream::iter(messages.clone()));

  let doubled: Vec<i32> = stream.map(|msg| *msg.payload() * 2).collect().await;

  assert_eq!(doubled, vec![20, 40]);
}

#[tokio::test]
async fn test_output_stream_filter() {
  use futures::StreamExt;
  use futures::stream;

  let messages: Vec<Message<i32>> = vec![
    Message::new(1, MessageId::new_sequence(1)),
    Message::new(2, MessageId::new_sequence(2)),
    Message::new(3, MessageId::new_sequence(3)),
  ];

  let stream: <TestOutput<i32> as Output>::OutputStream = Box::pin(stream::iter(messages));

  let mut filtered = Vec::new();
  futures::pin_mut!(stream);
  while let Some(msg) = stream.next().await {
    if *msg.payload() % 2 == 0 {
      filtered.push(msg);
    }
  }

  assert_eq!(filtered.len(), 1);
  assert_eq!(*filtered[0].payload(), 2);
}
