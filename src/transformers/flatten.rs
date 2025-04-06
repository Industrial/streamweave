use crate::error::TransformError;
use crate::traits::{input::Input, output::Output, transformer::Transformer};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error;
use std::fmt;
use std::pin::Pin;

pub struct FlattenTransformer<T> {
  _phantom: std::marker::PhantomData<T>,
}

impl<T> FlattenTransformer<T>
where
  T: Send + 'static,
{
  pub fn new() -> Self {
    Self {
      _phantom: std::marker::PhantomData,
    }
  }
}

#[derive(Debug)]
pub enum FlattenError {
  Transform(TransformError),
  EmptyInput,
}

impl fmt::Display for FlattenError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FlattenError::Transform(e) => write!(f, "{}", e),
      FlattenError::EmptyInput => write!(f, "Cannot flatten empty input vector"),
    }
  }
}

impl Error for FlattenError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      FlattenError::Transform(e) => Some(e),
      _ => None,
    }
  }
}

impl<T: Send + 'static> crate::traits::error::Error for FlattenTransformer<T> {
  type Error = FlattenError;
}

impl<T: Send + 'static> Input for FlattenTransformer<T> {
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<T: Send + 'static> Output for FlattenTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
}

#[async_trait]
impl<T> Transformer for FlattenTransformer<T>
where
  T: Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(futures::stream::unfold(input, |mut input| async move {
      while let Some(result) = input.next().await {
        match result {
          Ok(vec) => {
            if vec.is_empty() {
              continue;
            }
            // Create an iterator over the vector's items
            let mut items = vec.into_iter();
            // Get the first item
            if let Some(first) = items.next() {
              // Return the first item and store remaining items for next iteration
              let remaining = items.collect::<Vec<_>>();
              if !remaining.is_empty() {
                // Push remaining items back to input stream
                return Some((Ok(first), input));
              }
              return Some((Ok(first), input));
            }
          }
          Err(e) => return Some((Err(e), input)),
        }
      }
      None
    }))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::TryStreamExt;
  use futures::stream;

  // #[tokio::test]
  // async fn test_flatten_transformer() {
  //   let mut transformer = FlattenTransformer::new();
  //   let input = stream::iter(vec![vec![1, 2], vec![3, 4], vec![5]].into_iter().map(Ok));
  //   let boxed_input = Box::pin(input);
  //   let result: Vec<i32> = transformer
  //     .transform(boxed_input)
  //     .try_collect()
  //     .await
  //     .unwrap();
  //   assert_eq!(result, vec![1, 2, 3, 4, 5]);
  // }

  // #[tokio::test]
  // async fn test_flatten_transformer_empty_vectors() {
  //   let mut transformer = FlattenTransformer::new();
  //   let input = stream::iter(
  //     vec![vec![1], vec![], vec![2, 3], vec![]]
  //       .into_iter()
  //       .map(Ok),
  //   );
  //   let boxed_input = Box::pin(input);
  //   let result: Vec<i32> = transformer
  //     .transform(boxed_input)
  //     .try_collect()
  //     .await
  //     .unwrap();
  //   assert_eq!(result, vec![1, 2, 3]);
  // }

  #[tokio::test]
  async fn test_error_propagation() {
    let mut transformer = FlattenTransformer::new();
    let input = stream::iter(vec![
      Ok(vec![1, 2]),
      Err(FlattenError::Transform(TransformError::Custom(
        "test error".to_string(),
      ))),
      Ok(vec![3, 4]),
    ]);
    let boxed_input = Box::pin(input);

    let result = transformer
      .transform(boxed_input)
      .try_collect::<Vec<_>>()
      .await;

    assert!(result.is_err());
    match result {
      Err(FlattenError::Transform(e)) => assert_eq!(e.to_string(), "test error"),
      _ => panic!("Expected TransformError"),
    }
  }

  #[tokio::test]
  async fn test_empty_input_vectors() {
    let mut transformer = FlattenTransformer::new();
    let input = stream::iter(vec![Ok(vec![]), Ok(vec![]), Ok(vec![1]), Ok(vec![])]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1]);
  }
}
