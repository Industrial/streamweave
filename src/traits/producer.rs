use crate::traits::output::Output;

pub trait Producer: Output {
  fn produce(&mut self) -> Self::OutputStream;
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

  // Test producer implementation
  struct TestProducer {
    data: Vec<i32>,
  }

  impl Error for TestProducer {
    type Error = TestError;
  }

  impl Output for TestProducer {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  }

  impl Producer for TestProducer {
    fn produce(&mut self) -> Self::OutputStream {
      Box::pin(stream::iter(self.data.clone().into_iter().map(Ok)))
    }
  }

  #[tokio::test]
  async fn test_producer_stream() {
    let mut producer = TestProducer {
      data: vec![1, 2, 3],
    };

    let stream = producer.produce();
    let sum = stream
      .fold(0, |acc, x| async move { acc + x.unwrap() })
      .await;
    assert_eq!(sum, 6);
  }

  #[test]
  fn test_producer_trait_bounds() {
    fn takes_producer<P: Producer>(_: P) {}

    let producer = TestProducer {
      data: vec![1, 2, 3],
    };
    takes_producer(producer);
  }

  #[tokio::test]
  async fn test_producer_stream_send() {
    let mut producer = TestProducer {
      data: vec![1, 2, 3],
    };

    let stream = producer.produce();

    let handle = tokio::spawn(async move {
      let sum = stream
        .fold(0, |acc, x| async move { acc + x.unwrap() })
        .await;
      assert_eq!(sum, 6);
    });

    handle.await.unwrap();
  }

  #[test]
  fn test_producer_static_bounds() {
    fn returns_producer() -> impl Producer {
      TestProducer {
        data: vec![1, 2, 3],
      }
    }

    let _producer = returns_producer();
  }
}
