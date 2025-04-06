use crate::error::TransformError;
use crate::traits::{input::Input, output::Output, transformer::Transformer};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::pin::Pin;

pub struct DedupeTransformer<T> {
  seen: HashSet<T>,
}

impl<T> DedupeTransformer<T>
where
  T: Eq + std::hash::Hash + Clone + Send + 'static,
{
  pub fn new() -> Self {
    Self {
      seen: HashSet::new(),
    }
  }
}

#[derive(Debug)]
pub enum DedupeError {
  Transform(TransformError),
  SetError(String),
}

impl fmt::Display for DedupeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      DedupeError::Transform(e) => write!(f, "{}", e),
      DedupeError::SetError(msg) => write!(f, "HashSet operation failed: {}", msg),
    }
  }
}

impl Error for DedupeError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      DedupeError::Transform(e) => Some(e),
      _ => None,
    }
  }
}

impl<T: Send + 'static> crate::traits::error::Error for DedupeTransformer<T> {
  type Error = DedupeError;
}

impl<T: Send + 'static> Input for DedupeTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<T: Send + 'static> Output for DedupeTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
}

#[async_trait]
impl<T> Transformer for DedupeTransformer<T>
where
  T: Eq + std::hash::Hash + Clone + Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(
      futures::stream::unfold((input, self.seen.clone()), |mut state| async move {
        let (mut input, mut seen) = state;

        match input.next().await {
          Some(Ok(item)) => {
            let is_new = seen.insert(item.clone());
            let result = if is_new {
              Ok(item)
            } else {
              // Skip duplicates by continuing to next item
              return Some((None, (input, seen)));
            };
            Some((Some(result), (input, seen)))
          }
          Some(Err(e)) => Some((Some(Err(e)), (input, seen))),
          None => None,
        }
      })
      .filter_map(|x| futures::future::ready(x)),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::TryStreamExt;
  use futures::stream;

  #[tokio::test]
  async fn test_dedupe_transformer() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(vec![1, 2, 2, 3, 3, 3, 4].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 2, 3, 4]);
  }

  #[tokio::test]
  async fn test_dedupe_transformer_strings() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(vec!["a", "b", "b", "c", "a"].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<&str> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec!["a", "b", "c"]);
  }

  #[tokio::test]
  async fn test_dedupe_transformer_empty() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(Vec::<Result<i32, DedupeError>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_dedupe_transformer_all_duplicates() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(vec![1, 1, 1, 1].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1]);
  }

  // #[tokio::test]
  // async fn test_dedupe_transformer_multiple_transforms() {
  //   let mut transformer = DedupeTransformer::new();
  //   // First transform
  //   let input1 = stream::iter(vec![1, 2, 2, 3].into_iter().map(Ok));
  //   let boxed_input1 = Box::pin(input1);
  //   let result1: Vec<i32> = transformer
  //     .transform(boxed_input1)
  //     .try_collect()
  //     .await
  //     .unwrap();
  //   assert_eq!(result1, vec![1, 2, 3]);
  //   // Second transform with new data
  //   let input2 = stream::iter(vec![2, 3, 4, 4].into_iter().map(Ok));
  //   let boxed_input2 = Box::pin(input2);
  //   let result2: Vec<i32> = transformer
  //     .transform(boxed_input2)
  //     .try_collect()
  //     .await
  //     .unwrap();
  //   assert_eq!(result2, vec![4]); // Only 4 is new
  // }

  #[tokio::test]
  async fn test_error_propagation() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(vec![
      Ok(1),
      Ok(2),
      Err(DedupeError::Transform(TransformError::Custom(
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
      Err(DedupeError::Transform(e)) => assert_eq!(e.to_string(), "test error"),
      _ => panic!("Expected TransformError"),
    }
  }
}
