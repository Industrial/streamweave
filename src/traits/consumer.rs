use crate::traits::input::Input;
use async_trait::async_trait;

#[async_trait]
pub trait Consumer: Input {
  async fn consume(&mut self, stream: Self::InputStream) -> Result<(), Self::Error>;
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
  use std::sync::{Arc, Mutex};

  // Define test error type
  #[derive(Debug)]
  struct TestError(String);

  impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "Test error: {}", self.0)
    }
  }

  impl StdError for TestError {}

  // Mock consumer that collects items into a vector
  struct CollectorConsumer<T> {
    collected: Arc<Mutex<Vec<T>>>,
  }

  impl<T: Clone + Send + 'static> CollectorConsumer<T> {
    fn new() -> Self {
      Self {
        collected: Arc::new(Mutex::new(Vec::new())),
      }
    }

    fn get_collected(&self) -> Vec<T> {
      self.collected.lock().unwrap().clone()
    }
  }

  impl<T: Clone + Send + 'static> Error for CollectorConsumer<T> {
    type Error = TestError;
  }

  impl<T: Clone + Send + 'static> Input for CollectorConsumer<T> {
    type Input = T;
    type InputStream = Pin<Box<dyn Stream<Item = Result<T, TestError>> + Send>>;
  }

  #[async_trait]
  impl<T: Clone + Send + 'static> Consumer for CollectorConsumer<T> {
    async fn consume(&mut self, mut stream: Self::InputStream) -> Result<(), Self::Error> {
      while let Some(item) = stream.next().await {
        match item {
          Ok(value) => self.collected.lock().unwrap().push(value),
          Err(e) => return Err(e),
        }
      }
      Ok(())
    }
  }

  // Mock consumer that fails after N items
  struct FailingConsumer {
    fail_after: usize,
    count: usize,
  }

  impl FailingConsumer {
    fn new(fail_after: usize) -> Self {
      Self {
        fail_after,
        count: 0,
      }
    }
  }

  impl Error for FailingConsumer {
    type Error = TestError;
  }

  impl Input for FailingConsumer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  #[async_trait]
  impl Consumer for FailingConsumer {
    async fn consume(&mut self, mut stream: Self::InputStream) -> Result<(), Self::Error> {
      while let Some(item) = stream.next().await {
        item?; // Propagate any errors from the stream
        self.count += 1;
        if self.count >= self.fail_after {
          return Err(TestError("Intentional failure".into()));
        }
      }
      Ok(())
    }
  }

  #[tokio::test]
  async fn test_collector_consumer() {
    let mut consumer = CollectorConsumer::new();
    let input = vec![1, 2, 3, 4, 5];
    let stream = Box::pin(stream::iter(input.clone()).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_ok());
    assert_eq!(consumer.get_collected(), input);
  }

  #[tokio::test]
  async fn test_empty_stream() {
    let mut consumer = CollectorConsumer::<i32>::new();
    let input: Vec<i32> = vec![];
    let stream = Box::pin(stream::iter(input).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_ok());
    assert!(consumer.get_collected().is_empty());
  }

  #[tokio::test]
  async fn test_string_consumer() {
    let mut consumer = CollectorConsumer::new();
    let input = vec!["hello".to_string(), "world".to_string()];
    let stream = Box::pin(stream::iter(input.clone()).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_ok());
    assert_eq!(consumer.get_collected(), input);
  }

  #[tokio::test]
  async fn test_failing_consumer() {
    let mut consumer = FailingConsumer::new(3);
    let input = vec![1, 2, 3, 4, 5];
    let stream = Box::pin(stream::iter(input).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_err());
    if let Err(TestError(msg)) = result {
      assert_eq!(msg, "Intentional failure");
    } else {
      panic!("Expected TestError");
    }
  }

  #[tokio::test]
  async fn test_stream_error_propagation() {
    let mut consumer = CollectorConsumer::new();
    let input: Vec<Result<i32, TestError>> =
      vec![Ok(1), Ok(2), Err(TestError("test error".into())), Ok(4)];
    let stream = Box::pin(stream::iter(input));

    let result = consumer.consume(stream).await;
    assert!(result.is_err());
    assert_eq!(consumer.get_collected(), vec![1, 2]);
  }

  #[tokio::test]
  async fn test_multiple_consume_calls() {
    let mut consumer = CollectorConsumer::new();

    // First consume
    let input1 = vec![1, 2, 3];
    let stream1 = Box::pin(stream::iter(input1).map(Ok));
    let result1 = consumer.consume(stream1).await;
    assert!(result1.is_ok());

    // Second consume
    let input2 = vec![4, 5, 6];
    let stream2 = Box::pin(stream::iter(input2).map(Ok));
    let result2 = consumer.consume(stream2).await;
    assert!(result2.is_ok());

    // Check final state
    assert_eq!(consumer.get_collected(), vec![1, 2, 3, 4, 5, 6]);
  }
}
