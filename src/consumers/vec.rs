use crate::traits::{consumer::Consumer, error::Error, input::Input};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum VecError {
  LockError(String),
  PushError(String),
}

impl fmt::Display for VecError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      VecError::LockError(msg) => write!(f, "Failed to acquire lock: {}", msg),
      VecError::PushError(msg) => write!(f, "Failed to push item: {}", msg),
    }
  }
}

impl StdError for VecError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    None
  }
}

pub struct VecConsumer<T> {
  items: Arc<Mutex<Vec<T>>>,
}

impl<T> VecConsumer<T> {
  pub fn new() -> Self {
    Self {
      items: Arc::new(Mutex::new(Vec::new())),
    }
  }

  pub fn items(&self) -> Arc<Mutex<Vec<T>>> {
    self.items.clone()
  }
}

impl<T> Default for VecConsumer<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: Send + 'static> Error for VecConsumer<T> {
  type Error = VecError;
}

impl<T: Send + 'static> Input for VecConsumer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

#[async_trait]
impl<T: Send + 'static> Consumer for VecConsumer<T> {
  async fn consume(&mut self, mut stream: Self::InputStream) -> Result<(), Self::Error> {
    let mut vec = self
      .items
      .try_lock()
      .map_err(|e| VecError::LockError(format!("Failed to acquire lock: {}", e)))?;

    while let Some(item) = stream.next().await {
      vec.push(item.map_err(|e| VecError::PushError(format!("Failed to push item: {}", e)))?);
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_vec_consumer_integers() {
    let mut consumer = VecConsumer::new();
    let input = vec![1, 2, 3];
    let stream = Box::pin(stream::iter(input.clone()).map(Ok));

    consumer.consume(stream).await.unwrap();

    let binding = consumer.items();
    let items = binding.lock().await;
    assert_eq!(*items, input);
  }

  #[tokio::test]
  async fn test_vec_consumer_strings() {
    let mut consumer = VecConsumer::new();
    let input = vec!["hello".to_string(), "world".to_string()];
    let stream = Box::pin(stream::iter(input.clone()).map(Ok));

    consumer.consume(stream).await.unwrap();

    let binding = consumer.items();
    let items = binding.lock().await;
    assert_eq!(*items, input);
  }

  #[tokio::test]
  async fn test_vec_consumer_empty() {
    let mut consumer = VecConsumer::<i32>::new();
    let input: Vec<i32> = vec![];
    let stream = Box::pin(stream::iter(input.clone()).map(Ok));

    consumer.consume(stream).await.unwrap();

    let binding = consumer.items();
    let items = binding.lock().await;
    assert!(items.is_empty());
  }

  #[tokio::test]
  async fn test_multiple_consumes() {
    let mut consumer = VecConsumer::new();

    // First consume
    let input1 = vec![1, 2, 3];
    let stream1 = Box::pin(stream::iter(input1.clone()).map(Ok));
    consumer.consume(stream1).await.unwrap();

    // Second consume
    let input2 = vec![4, 5, 6];
    let stream2 = Box::pin(stream::iter(input2.clone()).map(Ok));
    consumer.consume(stream2).await.unwrap();

    let binding = consumer.items();
    let items = binding.lock().await;
    let expected: Vec<i32> = input1.into_iter().chain(input2).collect();
    assert_eq!(*items, expected);
  }

  // #[tokio::test]
  // async fn test_error_handling() {
  //   let mut consumer = VecConsumer::<i32>::new();
  //   let error_stream = Box::pin(stream::iter(vec![
  //     Ok(1),
  //     Err(VecError::PushError("test error".to_string())),
  //     Ok(3),
  //   ]));
  //   let result = consumer.consume(error_stream).await;
  //   assert!(result.is_err());
  //   match result {
  //     Err(VecError::PushError(msg)) => assert_eq!(msg, "Failed to push item: test error"),
  //     _ => panic!("Expected PushError"),
  //   }
  //   // Verify only items before error were stored
  //   let binding = consumer.items();
  //   let items = binding.lock().await;
  //   assert_eq!(*items, vec![1]);
  // }
}
