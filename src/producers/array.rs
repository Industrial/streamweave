use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;

#[derive(Debug)]
pub enum ArrayError {
  InvalidOperation(String),
}

impl fmt::Display for ArrayError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ArrayError::InvalidOperation(msg) => write!(f, "Array operation error: {}", msg),
    }
  }
}

impl StdError for ArrayError {}

pub struct ArrayProducer<T, const N: usize> {
  array: [T; N],
}

impl<T: Clone, const N: usize> ArrayProducer<T, N> {
  pub fn new(array: [T; N]) -> Self {
    Self { array }
  }

  pub fn from_slice(slice: &[T]) -> Option<Self>
  where
    T: Copy,
  {
    if slice.len() != N {
      return None;
    }
    let mut arr = std::mem::MaybeUninit::<[T; N]>::uninit();
    let ptr = arr.as_mut_ptr() as *mut T;
    unsafe {
      for (i, &item) in slice.iter().enumerate() {
        ptr.add(i).write(item);
      }
      Some(Self::new(arr.assume_init()))
    }
  }

  pub fn len(&self) -> usize {
    N
  }

  pub fn is_empty(&self) -> bool {
    N == 0
  }
}

impl<T: Clone + Send + 'static, const N: usize> Error for ArrayProducer<T, N> {
  type Error = ArrayError;
}

impl<T: Clone + Send + 'static, const N: usize> Output for ArrayProducer<T, N> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<T, ArrayError>> + Send>>;
}

impl<T: Clone + Send + 'static, const N: usize> Producer for ArrayProducer<T, N> {
  fn produce(&mut self) -> Self::OutputStream {
    let items = self.array.clone();
    Box::pin(futures::stream::iter(items.into_iter().map(Ok)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_array_producer() {
    let mut producer = ArrayProducer::new([1, 2, 3]);
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_array_producer_empty() {
    let mut producer = ArrayProducer::<i32, 0>::new([]);
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_array_producer_from_slice_success() {
    let slice = &[1, 2, 3];
    let mut producer = ArrayProducer::<_, 3>::from_slice(slice).unwrap();
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_array_producer_from_slice_failure() {
    let slice = &[1, 2, 3];
    assert!(ArrayProducer::<_, 4>::from_slice(slice).is_none());
  }

  #[tokio::test]
  async fn test_array_producer_multiple_calls() {
    let mut producer = ArrayProducer::new([1, 2, 3]);

    // First call
    let stream = producer.produce();
    let result1: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1, vec![1, 2, 3]);

    // Second call
    let stream = producer.produce();
    let result2: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_len_and_is_empty() {
    let producer: ArrayProducer<i32, 3> = ArrayProducer::new([1, 2, 3]);
    assert_eq!(producer.len(), 3);
    assert!(!producer.is_empty());

    let empty_producer: ArrayProducer<i32, 0> = ArrayProducer::new([]);
    assert_eq!(empty_producer.len(), 0);
    assert!(empty_producer.is_empty());
  }
}
