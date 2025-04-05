use crate::error::TransformError;
use crate::traits::{input::Input, output::Output, transformer::Transformer};
use async_trait::async_trait;
use futures::TryStreamExt;
use futures::{Stream, StreamExt};
use std::error::Error;
use std::fmt;
use std::pin::Pin;

pub struct BatchTransformer<T> {
  size: usize,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> BatchTransformer<T>
where
  T: Send + 'static,
{
  pub fn new(size: usize) -> Result<Self, BatchError> {
    if size == 0 {
      return Err(BatchError::BatchSizeZero);
    }
    Ok(Self {
      size,
      _phantom: std::marker::PhantomData,
    })
  }
}

#[derive(Debug)]
pub enum BatchError {
  Transform(TransformError),
  BatchSizeZero,
}

impl fmt::Display for BatchError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      BatchError::Transform(e) => write!(f, "{}", e),
      BatchError::BatchSizeZero => write!(f, "Batch size must be greater than zero"),
    }
  }
}

impl Error for BatchError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      BatchError::Transform(e) => Some(e),
      _ => None,
    }
  }
}

impl<T: Send + 'static> crate::traits::error::Error for BatchTransformer<T> {
  type Error = BatchError;
}

impl<T: Send + 'static> Input for BatchTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<T: Send + 'static> Output for BatchTransformer<T> {
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
}

#[async_trait]
impl<T: Send + 'static> Transformer for BatchTransformer<T> {
  fn transform(&mut self, mut input: Self::InputStream) -> Self::OutputStream {
    let size = self.size;
    let mut current_batch: Vec<T> = Vec::with_capacity(size);

    let stream = async_stream::try_stream! {
        while let Some(result) = input.next().await {
            let item = result?;
            current_batch.push(item);

            if current_batch.len() >= size {
                yield current_batch.split_off(0);
            }
        }

        // Emit any remaining items
        if !current_batch.is_empty() {
            yield current_batch;
        }
    };

    Box::pin(stream)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_batch_exact_size() {
    let mut transformer = BatchTransformer::new(3).unwrap();
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1, 2, 3], vec![4, 5, 6]]);
  }

  #[tokio::test]
  async fn test_batch_partial_last_chunk() {
    let mut transformer = BatchTransformer::new(2).unwrap();
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1, 2], vec![3, 4], vec![5]]);
  }

  #[tokio::test]
  async fn test_batch_empty_input() {
    let mut transformer = BatchTransformer::new(2).unwrap();
    let input = stream::iter(Vec::<Result<i32, BatchError>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_batch_size_one() {
    let mut transformer = BatchTransformer::new(1).unwrap();
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1], vec![2], vec![3]]);
  }

  #[tokio::test]
  async fn test_batch_size_larger_than_input() {
    let mut transformer = BatchTransformer::new(5).unwrap();
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1, 2, 3]]);
  }

  #[tokio::test]
  async fn test_error_propagation() {
    let mut transformer = BatchTransformer::new(2).unwrap();
    let input = stream::iter(vec![
      Ok(1),
      Ok(2),
      Err(BatchError::Transform(TransformError::Custom(
        "test error".to_string(),
      ))),
      Ok(4),
    ]);
    let boxed_input = Box::pin(input);

    let result = transformer
      .transform(boxed_input)
      .try_collect::<Vec<_>>()
      .await;

    assert!(result.is_err());
    match result {
      Err(BatchError::Transform(e)) => assert_eq!(e.to_string(), "test error"),
      _ => panic!("Expected TransformError"),
    }
  }
}
