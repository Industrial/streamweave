use crate::traits::{consumer::Consumer, error::Error, input::Input};
use async_trait::async_trait;
use futures::StreamExt;
use std::pin::Pin;

#[derive(Debug)]
pub struct ArrayConsumer<T, const N: usize> {
  array: [Option<T>; N],
  current_index: usize,
}

#[derive(Debug)]
pub enum ArrayConsumerError {
  ArrayFull,
}

impl std::fmt::Display for ArrayConsumerError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ArrayConsumerError::ArrayFull => write!(f, "Array is full"),
    }
  }
}

impl std::error::Error for ArrayConsumerError {}

impl<T, const N: usize> Error for ArrayConsumer<T, N> {
  type Error = ArrayConsumerError;
}

impl<T, const N: usize> Input for ArrayConsumer<T, N> {
  type Input = T;
  type InputStream = Pin<Box<dyn futures::Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<T, const N: usize> ArrayConsumer<T, N> {
  pub fn new() -> Self {
    Self {
      array: [(); N].map(|_| None),
      current_index: 0,
    }
  }

  pub fn into_inner(self) -> [Option<T>; N] {
    self.array
  }

  pub fn len(&self) -> usize {
    self.current_index
  }

  pub fn is_empty(&self) -> bool {
    self.current_index == 0
  }

  pub fn is_full(&self) -> bool {
    self.current_index >= N
  }

  pub fn remaining_capacity(&self) -> usize {
    N - self.current_index
  }
}

impl<T, const N: usize> Default for ArrayConsumer<T, N> {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl<T, const N: usize> Consumer for ArrayConsumer<T, N>
where
  T: Send,
{
  async fn consume(&mut self, mut input: Self::InputStream) -> Result<(), Self::Error> {
    while let Some(result) = input.next().await {
      if self.is_full() {
        return Err(ArrayConsumerError::ArrayFull);
      }
      let item = result?;
      self.array[self.current_index] = Some(item);
      self.current_index += 1;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use tokio;

  #[tokio::test]
  async fn test_empty_stream() {
    let mut consumer = ArrayConsumer::<i32, 5>::new();
    let input = stream::empty();
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert!(consumer.is_empty());
    assert_eq!(consumer.len(), 0);
    assert_eq!(consumer.remaining_capacity(), 5);
  }

  #[tokio::test]
  async fn test_full_capacity() {
    let mut consumer = ArrayConsumer::<i32, 3>::new();
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert!(!consumer.is_empty());
    assert!(consumer.is_full());
    assert_eq!(consumer.len(), 3);
    assert_eq!(consumer.remaining_capacity(), 0);

    let array = consumer.into_inner();
    assert_eq!(array, [Some(1), Some(2), Some(3)]);
  }

  #[tokio::test]
  async fn test_overflow() {
    let mut consumer = ArrayConsumer::<i32, 2>::new();
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ArrayConsumerError::ArrayFull));
    assert!(!consumer.is_empty());
    assert!(consumer.is_full());
    assert_eq!(consumer.len(), 2);
    assert_eq!(consumer.remaining_capacity(), 0);

    let array = consumer.into_inner();
    assert_eq!(array, [Some(1), Some(2)]);
  }

  #[tokio::test]
  async fn test_partial_fill() {
    let mut consumer = ArrayConsumer::<i32, 5>::new();
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert!(!consumer.is_empty());
    assert!(!consumer.is_full());
    assert_eq!(consumer.len(), 3);
    assert_eq!(consumer.remaining_capacity(), 2);

    let array = consumer.into_inner();
    assert_eq!(array, [Some(1), Some(2), Some(3), None, None]);
  }

  #[tokio::test]
  async fn test_with_error() {
    let mut consumer = ArrayConsumer::<i32, 5>::new();
    let input = stream::iter(vec![Ok(1), Err(ArrayConsumerError::ArrayFull), Ok(3)]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ArrayConsumerError::ArrayFull));
    assert!(!consumer.is_empty());
    assert_eq!(consumer.len(), 1);

    let array = consumer.into_inner();
    assert_eq!(array[0], Some(1));
  }

  #[tokio::test]
  async fn test_zero_capacity() {
    let mut consumer = ArrayConsumer::<i32, 0>::new();
    let input = stream::iter(vec![Ok(1)]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ArrayConsumerError::ArrayFull));
    assert!(consumer.is_empty());
    assert!(consumer.is_full());
    assert_eq!(consumer.len(), 0);
    assert_eq!(consumer.remaining_capacity(), 0);
  }

  #[tokio::test]
  async fn test_large_capacity() {
    const CAPACITY: usize = 1000;
    let mut consumer = ArrayConsumer::<i32, CAPACITY>::new();
    let values: Vec<_> = (0..CAPACITY).map(|i| Ok(i as i32)).collect();
    let input = stream::iter(values);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert!(!consumer.is_empty());
    assert!(consumer.is_full());
    assert_eq!(consumer.len(), CAPACITY);
    assert_eq!(consumer.remaining_capacity(), 0);

    let array = consumer.into_inner();
    for i in 0..CAPACITY {
      assert_eq!(array[i], Some(i as i32));
    }
  }

  #[tokio::test]
  async fn test_string_array() {
    let mut consumer = ArrayConsumer::<String, 3>::new();
    let input = stream::iter(vec![
      Ok("hello".to_string()),
      Ok("world".to_string()),
      Ok("!".to_string()),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), 3);

    let array = consumer.into_inner();
    assert_eq!(
      array,
      [
        Some("hello".to_string()),
        Some("world".to_string()),
        Some("!".to_string()),
      ]
    );
  }

  #[tokio::test]
  async fn test_option_array() {
    let mut consumer = ArrayConsumer::<Option<i32>, 3>::new();
    let input = stream::iter(vec![Ok(Some(1)), Ok(None), Ok(Some(3))]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), 3);

    let array = consumer.into_inner();
    assert_eq!(array, [Some(Some(1)), Some(None), Some(Some(3))]);
  }

  #[tokio::test]
  async fn test_default_impl() {
    let consumer = ArrayConsumer::<i32, 5>::default();
    assert!(consumer.is_empty());
    assert!(!consumer.is_full());
    assert_eq!(consumer.len(), 0);
    assert_eq!(consumer.remaining_capacity(), 5);
  }
}
