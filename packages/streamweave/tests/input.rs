//! Tests for Input trait

use futures::{StreamExt, stream};
use std::pin::Pin;
use streamweave::Input;

// Test implementation using Vec<i32>
struct VecInput {
  data: Vec<i32>,
}

impl Input for VecInput {
  type Input = i32;
  type InputStream = Pin<Box<dyn futures::Stream<Item = i32> + Send>>;
}

// Test implementation using String
struct StringInput {
  data: String,
}

impl Input for StringInput {
  type Input = char;
  type InputStream = Pin<Box<dyn futures::Stream<Item = char> + Send>>;
}

#[tokio::test]
async fn test_vec_input_stream() {
  let input = VecInput {
    data: vec![1, 2, 3],
  };

  // Verify type constraints are met
  let stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> = Box::pin(stream::iter(input.data));

  let sum = stream.fold(0, |acc, x| async move { acc + x }).await;
  assert_eq!(sum, 6);
}

#[tokio::test]
async fn test_string_input_stream() {
  let input = StringInput {
    data: "hello".to_string(),
  };

  // Verify type constraints are met
  let stream: Pin<Box<dyn futures::Stream<Item = char> + Send>> =
    Box::pin(stream::iter(input.data.chars()));

  let result: String = stream.collect().await;
  assert_eq!(result, "hello");
}

#[tokio::test]
async fn test_input_stream_send() {
  let input = VecInput {
    data: vec![1, 2, 3],
  };

  let stream: Pin<Box<dyn futures::Stream<Item = i32> + Send>> = Box::pin(stream::iter(input.data));

  let handle = tokio::spawn(async move {
    let sum = stream.fold(0, |acc, x| async move { acc + x }).await;
    assert_eq!(sum, 6);
  });

  handle.await.unwrap();
}

#[test]
fn test_input_trait_bounds() {
  fn takes_input<T: Input>(_: T) {}

  takes_input(VecInput { data: vec![] });
  takes_input(StringInput {
    data: String::new(),
  });
}

#[test]
fn test_input_static_bounds() {
  fn static_input_fn() -> impl Input {
    VecInput { data: vec![] }
  }

  let _input = static_input_fn();
}
