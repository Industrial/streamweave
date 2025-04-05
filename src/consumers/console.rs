use crate::traits::{consumer::Consumer, error::Error, input::Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::pin::Pin;

#[derive(Debug)]
pub enum ConsoleError {
  WriteError(String),
}

impl fmt::Display for ConsoleError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ConsoleError::WriteError(msg) => write!(f, "Console write error: {}", msg),
    }
  }
}

impl StdError for ConsoleError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    None
  }
}

pub struct ConsoleConsumer<T> {
  _phantom: std::marker::PhantomData<T>,
}

impl<T> ConsoleConsumer<T> {
  pub fn new() -> Self {
    Self {
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T> Default for ConsoleConsumer<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: Display + Send + 'static> Error for ConsoleConsumer<T> {
  type Error = ConsoleError;
}

impl<T: Display + Send + 'static> Input for ConsoleConsumer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

#[async_trait]
impl<T: Display + Send + 'static> Consumer for ConsoleConsumer<T> {
  async fn consume(&mut self, mut stream: Self::InputStream) -> Result<(), Self::Error> {
    while let Some(item) = stream.next().await {
      println!("Received value: {}", item?);
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_console_consumer_integers() {
    let mut consumer = ConsoleConsumer::<i32>::new();
    let input = vec![1, 2, 3];
    let stream = Box::pin(stream::iter(input).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_console_consumer_strings() {
    let mut consumer = ConsoleConsumer::<String>::new();
    let input = vec!["hello".to_string(), "world".to_string()];
    let stream = Box::pin(stream::iter(input).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_console_consumer_floats() {
    let mut consumer = ConsoleConsumer::<f64>::new();
    let input = vec![1.1, 2.2, 3.3];
    let stream = Box::pin(stream::iter(input).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_console_consumer_custom_type() {
    #[derive(Debug)]
    struct CustomType(i32);

    impl Display for CustomType {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Custom({})", self.0)
      }
    }

    let mut consumer = ConsoleConsumer::<CustomType>::new();
    let input = vec![CustomType(1), CustomType(2), CustomType(3)];
    let stream = Box::pin(stream::iter(input).map(Ok));

    let result = consumer.consume(stream).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_console_consumer_reuse() {
    let mut consumer = ConsoleConsumer::<i32>::new();

    // First consumption
    let input1 = vec![1, 2, 3];
    let stream1 = Box::pin(stream::iter(input1).map(Ok));
    assert!(consumer.consume(stream1).await.is_ok());

    // Second consumption - should work fine
    let input2 = vec![4, 5, 6];
    let stream2 = Box::pin(stream::iter(input2).map(Ok));
    assert!(consumer.consume(stream2).await.is_ok());
  }

  #[tokio::test]
  async fn test_error_propagation() {
    let mut consumer = ConsoleConsumer::<i32>::new();
    let error_stream = Box::pin(stream::iter(vec![
      Ok(1),
      Err(ConsoleError::WriteError("test error".to_string())),
      Ok(3),
    ]));

    let result = consumer.consume(error_stream).await;
    assert!(result.is_err());
    if let Err(ConsoleError::WriteError(msg)) = result {
      assert_eq!(msg, "test error");
    } else {
      panic!("Expected WriteError");
    }
  }
}
