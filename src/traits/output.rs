use crate::traits::error::Error;
use futures::Stream;

pub trait Output: Error {
  type Output;
  type OutputStream: Stream<Item = Result<Self::Output, Self::Error>> + Send;
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

  // Test implementation for numeric output
  struct NumberOutput {
    data: Vec<i32>,
  }

  impl Error for NumberOutput {
    type Error = TestError;
  }

  impl Output for NumberOutput {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  // Test implementation for string output
  struct TextOutput {
    data: String,
  }

  impl Error for TextOutput {
    type Error = TestError;
  }

  impl Output for TextOutput {
    type Output = String;
    type OutputStream = Pin<Box<dyn Stream<Item = Result<String, TestError>> + Send>>;
  }

  #[tokio::test]
  async fn test_number_output_stream() {
    let output = NumberOutput {
      data: vec![1, 2, 3],
    };

    // Verify type constraints are met
    let stream: Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>> =
      Box::pin(stream::iter(output.data).map(Ok));

    // Test stream functionality
    let sum = stream
      .fold(0, |acc, x| async move { acc + x.unwrap() })
      .await;
    assert_eq!(sum, 6);
  }

  #[tokio::test]
  async fn test_text_output_stream() {
    let output = TextOutput {
      data: "hello".to_string(),
    };

    // Verify type constraints are met
    let stream: Pin<Box<dyn Stream<Item = Result<String, TestError>> + Send>> =
      Box::pin(stream::iter(output.data.chars().map(|c| Ok(c.to_string()))));

    // Test stream functionality
    let result = stream.map(|r| r.unwrap()).collect::<String>().await;
    assert_eq!(result, "hello");
  }

  #[tokio::test]
  async fn test_output_stream_send() {
    let output = NumberOutput {
      data: vec![1, 2, 3],
    };

    let stream: Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>> =
      Box::pin(stream::iter(output.data).map(Ok));

    // Verify we can send the stream between threads
    let handle = tokio::spawn(async move {
      let sum = stream
        .fold(0, |acc, x| async move { acc + x.unwrap() })
        .await;
      assert_eq!(sum, 6);
    });

    handle.await.unwrap();
  }

  #[test]
  fn test_output_trait_bounds() {
    fn takes_output<T: Output>(_: T) {}

    takes_output(NumberOutput { data: vec![] });
    takes_output(TextOutput {
      data: String::new(),
    });
  }

  #[test]
  fn test_output_static_bounds() {
    fn static_output_fn() -> impl Output {
      NumberOutput { data: vec![] }
    }

    let _output = static_output_fn();
  }

  #[test]
  fn test_output_type_inference() {
    // Test type inference works correctly with the Output trait
    fn get_output_type<T: Output>(output: T) -> T::Output {
      unimplemented!() // Just for type checking
    }

    let output = NumberOutput { data: vec![] };
    let _: i32 = get_output_type(output);

    let output = TextOutput {
      data: String::new(),
    };
    let _: String = get_output_type(output);
  }

  #[tokio::test]
  async fn test_output_stream_composition() {
    let output1 = NumberOutput {
      data: vec![1, 2, 3],
    };
    let output2 = NumberOutput {
      data: vec![4, 5, 6],
    };

    // Test that we can compose output streams
    let stream1: Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>> =
      Box::pin(stream::iter(output1.data).map(Ok));
    let stream2: Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>> =
      Box::pin(stream::iter(output2.data).map(Ok));

    let combined = Box::pin(stream::select(stream1, stream2));
    let sum = combined
      .fold(0, |acc, x| async move { acc + x.unwrap() })
      .await;
    assert_eq!(sum, 21); // 1 + 2 + 3 + 4 + 5 + 6
  }
}
