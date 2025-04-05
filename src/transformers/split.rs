use crate::error::TransformError;
use crate::traits::{input::Input, output::Output, transformer::Transformer};
use async_trait::async_trait;
use futures::stream;
use futures::{Stream, StreamExt};
use std::error::Error;
use std::fmt;
use std::pin::Pin;

#[derive(Debug)]
pub enum SplitError {
  Transform(TransformError),
  ChunkingFailed,
  PredicateError(String),
}

impl fmt::Display for SplitError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      SplitError::Transform(e) => write!(f, "{}", e),
      SplitError::ChunkingFailed => write!(f, "Failed to split stream into chunks"),
      SplitError::PredicateError(msg) => write!(f, "Predicate evaluation failed: {}", msg),
    }
  }
}

impl Error for SplitError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      SplitError::Transform(e) => Some(e),
      _ => None,
    }
  }
}

pub struct SplitTransformer<F, T> {
  predicate: F,
  _phantom: std::marker::PhantomData<T>,
}

impl<F, T> SplitTransformer<F, T>
where
  F: FnMut(&T) -> Result<bool, Box<dyn Error + Send + Sync>> + Send + Clone + 'static,
  T: Clone + Send + 'static,
{
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<F, T> crate::traits::error::Error for SplitTransformer<F, T>
where
  F: Send + 'static,
  T: Send + 'static,
{
  type Error = SplitError;
}

impl<F, T> Input for SplitTransformer<F, T>
where
  F: Send + 'static,
  T: Send + 'static,
{
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<F, T> Output for SplitTransformer<F, T>
where
  F: Send + 'static,
  T: Send + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
}

#[async_trait]
impl<F, T> Transformer for SplitTransformer<F, T>
where
  F: FnMut(&T) -> Result<bool, Box<dyn Error + Send + Sync>> + Send + Clone + 'static,
  T: Clone + Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let predicate = self.predicate.clone();

    Box::pin(futures::stream::unfold(
      (input, predicate),
      |(mut input, mut pred)| async move {
        if let Some(items_result) = input.next().await {
          match items_result {
            Ok(items) => {
              let mut current_chunk = Vec::new();
              let mut chunks = Vec::new();

              for item in items {
                match pred(&item) {
                  Ok(should_split) => {
                    if should_split && !current_chunk.is_empty() {
                      chunks.push(Ok(std::mem::take(&mut current_chunk)));
                    }
                    current_chunk.push(item);
                  }
                  Err(e) => {
                    return Some((
                      Err(SplitError::Transform(TransformError::OperationFailed(e))),
                      (input, pred),
                    ));
                  }
                }
              }

              if !current_chunk.is_empty() {
                chunks.push(Ok(current_chunk));
              }

              if chunks.is_empty() {
                Some((Ok(Vec::new()), (input, pred)))
              } else {
                let mut iter = chunks.into_iter();
                let first = iter.next().unwrap();
                Some((
                  first,
                  (
                    Box::pin(stream::iter(iter).chain(input))
                      as Pin<Box<dyn Stream<Item = Result<Vec<T>, SplitError>> + Send>>,
                    pred,
                  ),
                ))
              }
            }
            Err(e) => Some((Err(e), (input, pred))),
          }
        } else {
          None
        }
      },
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::TryStreamExt;
  use futures::stream;

  #[tokio::test]
  async fn test_split_by_even_numbers() {
    let mut transformer = SplitTransformer::new(|x: &i32| Ok(x % 2 == 0));
    let input = vec![vec![1, 2, 3, 4, 5, 6]];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<Vec<i32>> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1], vec![2, 3], vec![4, 5], vec![6]]);
  }

  #[tokio::test]
  async fn test_split_empty_input() {
    let mut transformer = SplitTransformer::new(|x: &i32| Ok(x % 2 == 0));
    let input = vec![Vec::<i32>::new()];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<Vec<i32>> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_split_no_splits() {
    let mut transformer = SplitTransformer::new(|x: &i32| Ok(*x < 0)); // No negatives in input
    let input = vec![vec![1, 2, 3, 4, 5]];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<Vec<i32>> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1, 2, 3, 4, 5]]);
  }

  #[tokio::test]
  async fn test_split_with_errors() {
    let mut transformer = SplitTransformer::new(|x: &i32| {
      if *x == 3 {
        Err("Cannot process 3".into())
      } else {
        Ok(x % 2 == 0)
      }
    });

    let input = vec![vec![1, 2, 3, 4, 5]];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result = transformer
      .transform(input_stream)
      .try_collect::<Vec<_>>()
      .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      SplitError::Transform(TransformError::OperationFailed(e)) => {
        assert_eq!(e.to_string(), "Cannot process 3")
      }
      _ => panic!("Expected OperationFailed error"),
    }
  }

  #[tokio::test]
  async fn test_split_strings() {
    let mut transformer = SplitTransformer::new(|s: &String| Ok(s.is_empty()));
    let input = vec![vec![
      "hello".to_string(),
      "".to_string(),
      "world".to_string(),
      "".to_string(),
      "rust".to_string(),
    ]];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<Vec<String>> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(
      result,
      vec![
        vec!["hello".to_string()],
        vec!["".to_string(), "world".to_string()],
        vec!["".to_string(), "rust".to_string()],
      ]
    );
  }

  #[tokio::test]
  async fn test_split_reuse() {
    let mut transformer = SplitTransformer::new(|x: &i32| Ok(x % 2 == 0));

    let input1 = vec![vec![1, 2, 3]];
    let input_stream1 = Box::pin(stream::iter(input1.into_iter().map(Ok)));
    let result1: Vec<Vec<i32>> = transformer
      .transform(input_stream1)
      .try_collect()
      .await
      .unwrap();
    assert_eq!(result1, vec![vec![1], vec![2, 3]]);

    let input2 = vec![vec![4, 5, 6]];
    let input_stream2 = Box::pin(stream::iter(input2.into_iter().map(Ok)));
    let result2: Vec<Vec<i32>> = transformer
      .transform(input_stream2)
      .try_collect()
      .await
      .unwrap();
    assert_eq!(result2, vec![vec![4, 5], vec![6]]);
  }
}
