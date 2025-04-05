use crate::traits::input::Input;
use crate::traits::output::Output;

pub trait Transformer: Input + Output {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::error::Error;
  use futures::Stream;
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

  // Mock transformer that doubles numbers
  struct DoubleTransformer;

  impl Error for DoubleTransformer {
    type Error = TestError;
  }

  impl Input for DoubleTransformer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  impl Output for DoubleTransformer {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  impl Transformer for DoubleTransformer {
    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
      Box::pin(input.map(|r| r.map(|x| x * 2)))
    }
  }

  // Mock transformer that converts numbers to strings
  struct ToStringTransformer;

  impl Error for ToStringTransformer {
    type Error = TestError;
  }

  impl Input for ToStringTransformer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  impl Output for ToStringTransformer {
    type Output = String;
    type OutputStream = Pin<Box<dyn Stream<Item = Result<String, TestError>> + Send>>;
  }

  impl Transformer for ToStringTransformer {
    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
      Box::pin(input.map(|r| r.map(|x| x.to_string())))
    }
  }

  #[tokio::test]
  async fn test_double_transformer() {
    let mut transformer = DoubleTransformer;
    let input = vec![1, 2, 3, 4, 5];
    let input_stream = Box::pin(stream::iter(input).map(Ok));

    let output_stream = transformer.transform(input_stream);
    let result: Vec<i32> = output_stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result, vec![2, 4, 6, 8, 10]);
  }

  #[tokio::test]
  async fn test_double_transformer_empty_stream() {
    let mut transformer = DoubleTransformer;
    let input: Vec<i32> = vec![];
    let input_stream = Box::pin(stream::iter(input).map(Ok));

    let output_stream = transformer.transform(input_stream);
    let result: Vec<i32> = output_stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_string_transformer() {
    let mut transformer = ToStringTransformer;
    let input = vec![1, 2, 3];
    let input_stream = Box::pin(stream::iter(input).map(Ok));

    let output_stream = transformer.transform(input_stream);
    let result: Vec<String> = output_stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result, vec!["1", "2", "3"]);
  }

  #[tokio::test]
  async fn test_chained_transformers() {
    let mut double_transformer = DoubleTransformer;
    let mut string_transformer = ToStringTransformer;

    let input = vec![1, 2, 3];
    let input_stream = Box::pin(stream::iter(input).map(Ok));

    // Chain transformers: numbers -> doubled -> strings
    let doubled_stream = double_transformer.transform(input_stream);
    let final_stream = string_transformer.transform(doubled_stream);

    let result: Vec<String> = final_stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["2", "4", "6"]);
  }

  #[tokio::test]
  async fn test_transformer_error_propagation() {
    let mut double_transformer = DoubleTransformer;
    let input = vec![Ok(1), Err(TestError("test error".to_string())), Ok(3)];
    let input_stream = Box::pin(stream::iter(input));

    let output_stream = double_transformer.transform(input_stream);
    let result: Vec<Result<i32, _>> = output_stream.collect().await;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0].as_ref().unwrap(), &2);
    assert!(result[1].is_err());
    assert_eq!(result[2].as_ref().unwrap(), &6);
  }
}
