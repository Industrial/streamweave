use crate::input::Input;
use crate::message::{Message, MessageId};
use futures::Stream;
use std::pin::Pin;

// Test implementation of Input trait
struct TestInput<T>
where
  T: Send + 'static,
{
  _phantom: std::marker::PhantomData<T>,
}

impl<T> Input for TestInput<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = Message<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

// ============================================================================
// Trait Implementation Tests
// ============================================================================

#[test]
fn test_input_trait_impl_i32() {
  let _input: TestInput<i32> = TestInput {
    _phantom: std::marker::PhantomData,
  };
  // Just verify it compiles and implements the trait
}

#[test]
fn test_input_trait_impl_string() {
  let _input: TestInput<String> = TestInput {
    _phantom: std::marker::PhantomData,
  };
  // Just verify it compiles and implements the trait
}

#[test]
fn test_input_trait_impl_bool() {
  let _input: TestInput<bool> = TestInput {
    _phantom: std::marker::PhantomData,
  };
  // Just verify it compiles and implements the trait
}

// ============================================================================
// Type Constraints Tests
// ============================================================================

#[test]
fn test_input_type_is_send() {
  // Verify that Input::Input must be Send
  fn assert_send<T: Send>() {}

  assert_send::<Message<i32>>();
  assert_send::<Message<String>>();
  assert_send::<Message<bool>>();
}

#[test]
fn test_input_stream_is_send() {
  use futures::stream;

  // Verify that InputStream must be Send
  fn assert_stream_is_send<S: Stream<Item = Message<i32>> + Send>() {}

  let stream = stream::empty::<Message<i32>>();
  assert_stream_is_send::<Pin<Box<dyn Stream<Item = Message<i32>> + Send>>>();

  let _boxed: Pin<Box<dyn Stream<Item = Message<i32>> + Send>> = Box::pin(stream);
}

// ============================================================================
// Generic Input Tests
// ============================================================================

#[test]
fn test_input_with_different_types() {
  // Test that Input trait works with various types
  let _input_i32: TestInput<i32> = TestInput {
    _phantom: std::marker::PhantomData,
  };

  let _input_string: TestInput<String> = TestInput {
    _phantom: std::marker::PhantomData,
  };

  let _input_vec: TestInput<Vec<i32>> = TestInput {
    _phantom: std::marker::PhantomData,
  };
}

// ============================================================================
// Associated Type Tests
// ============================================================================

#[test]
fn test_input_associated_type() {
  // Verify that Input::Input is Message<T>
  fn assert_input_type<T>() -> std::marker::PhantomData<T> {
    std::marker::PhantomData
  }

  let _phantom: std::marker::PhantomData<Message<i32>> = assert_input_type::<Message<i32>>();
}

#[test]
fn test_input_stream_type() {
  // Verify that InputStream yields Input::Input
  use futures::stream;

  let _stream: Pin<Box<dyn Stream<Item = Message<i32>> + Send>> =
    Box::pin(stream::empty::<Message<i32>>());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_input_trait_constraints() {
  // Verify trait bounds work correctly
  fn take_input<I>(_input: I)
  where
    I: Input,
    I::Input: Send + 'static,
  {
  }

  let test_input = TestInput::<i32> {
    _phantom: std::marker::PhantomData,
  };
  take_input(test_input);
}

#[tokio::test]
async fn test_input_stream_consumption() {
  use futures::StreamExt;
  use futures::stream;

  let _test_input = TestInput::<i32> {
    _phantom: std::marker::PhantomData,
  };

  // Create a stream that matches the Input trait
  let messages: Vec<Message<i32>> = vec![
    Message::new(1, MessageId::new_sequence(1)),
    Message::new(2, MessageId::new_sequence(2)),
    Message::new(3, MessageId::new_sequence(3)),
  ];

  let stream: <TestInput<i32> as Input>::InputStream = Box::pin(stream::iter(messages.clone()));

  let collected: Vec<Message<i32>> = stream.collect().await;
  assert_eq!(collected, messages);
}

#[tokio::test]
async fn test_input_stream_empty() {
  use futures::StreamExt;
  use futures::stream;

  let stream: <TestInput<i32> as Input>::InputStream = Box::pin(stream::empty::<Message<i32>>());

  let collected: Vec<Message<i32>> = stream.collect().await;
  assert!(collected.is_empty());
}

#[tokio::test]
async fn test_input_stream_single_item() {
  use futures::StreamExt;
  use futures::stream;

  let message = Message::new(42, MessageId::new_sequence(1));
  let stream: <TestInput<i32> as Input>::InputStream =
    Box::pin(stream::iter(vec![message.clone()]));

  let collected: Vec<Message<i32>> = stream.collect().await;
  assert_eq!(collected.len(), 1);
  assert_eq!(collected[0].payload(), message.payload());
}
