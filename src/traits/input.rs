use crate::traits::error::Error;
use futures::Stream;

pub trait Input: Error {
  type Input;
  type InputStream: Stream<Item = Result<Self::Input, Self::Error>> + Send;
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::{StreamExt, stream};
  use std::error::Error as StdError;
  use std::fmt;
  use std::pin::Pin;

  // Test error type
  #[derive(Debug)]
  struct TestError(String);

  impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "Test error: {}", self.0)
    }
  }

  impl StdError for TestError {}

  // Test implementation using Vec<i32>
  struct VecInput {
    data: Vec<i32>,
  }

  impl Error for VecInput {
    type Error = TestError;
  }

  impl Input for VecInput {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  // Test implementation using String
  struct StringInput {
    data: String,
  }

  impl Error for StringInput {
    type Error = TestError;
  }

  impl Input for StringInput {
    type Input = char;
    type InputStream = Pin<Box<dyn Stream<Item = Result<char, TestError>> + Send>>;
  }

  #[tokio::test]
  async fn test_vec_input_stream() {
    let input = VecInput {
      data: vec![1, 2, 3],
    };

    // Verify type constraints are met
    let stream: Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>> =
      Box::pin(stream::iter(input.data).map(Ok));

    let sum = stream
      .fold(0, |acc, x| async move { acc + x.unwrap() })
      .await;
    assert_eq!(sum, 6);
  }

  #[tokio::test]
  async fn test_string_input_stream() {
    let input = StringInput {
      data: "hello".to_string(),
    };

    // Verify type constraints are met
    let stream: Pin<Box<dyn Stream<Item = Result<char, TestError>> + Send>> =
      Box::pin(stream::iter(input.data.chars()).map(Ok));

    let result: String = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, "hello");
  }

  #[tokio::test]
  async fn test_input_stream_send() {
    let input = VecInput {
      data: vec![1, 2, 3],
    };

    let stream: Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>> =
      Box::pin(stream::iter(input.data).map(Ok));

    let handle = tokio::spawn(async move {
      let sum = stream
        .fold(0, |acc, x| async move { acc + x.unwrap() })
        .await;
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
}
