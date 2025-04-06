use crate::error::TransformError;
use crate::traits::{input::Input, output::Output, transformer::Transformer};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error;
use std::fmt;
use std::pin::Pin;

pub struct MergeTransformer<T> {
  _phantom: std::marker::PhantomData<T>,
}

impl<T> MergeTransformer<T>
where
  T: Send + 'static,
{
  pub fn new() -> Self {
    Self {
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T> Default for MergeTransformer<T>
where
  T: Send + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

#[derive(Debug)]
pub enum MergeError {
  Transform(TransformError),
  EmptyInput,
  EmptyStream,
}

impl fmt::Display for MergeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      MergeError::Transform(e) => write!(f, "{}", e),
      MergeError::EmptyInput => write!(f, "No streams provided to merge"),
      MergeError::EmptyStream => write!(f, "Stream contained no items"),
    }
  }
}

impl Error for MergeError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      MergeError::Transform(e) => Some(e),
      _ => None,
    }
  }
}

impl<T: Send + 'static> crate::traits::error::Error for MergeTransformer<T> {
  type Error = MergeError;
}

impl<T: Send + 'static> Input for MergeTransformer<T> {
  type Input = Vec<Pin<Box<dyn Stream<Item = Result<T, MergeError>> + Send>>>;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<T: Send + 'static> Output for MergeTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
}

#[async_trait]
impl<T> Transformer for MergeTransformer<T>
where
  T: Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(futures::stream::unfold(input, |mut input| async move {
      while let Some(streams_result) = input.next().await {
        match streams_result {
          Ok(streams) => {
            if streams.is_empty() {
              continue;
            }

            let mut all_items = Vec::new();
            for mut stream in streams {
              while let Some(item_result) = stream.next().await {
                match item_result {
                  Ok(item) => all_items.push(item),
                  Err(e) => return Some((Err(e), input)),
                }
              }
            }

            // Process items one at a time
            if !all_items.is_empty() {
              let item = all_items.remove(0);
              // If there are remaining items, create a new stream with them
              if !all_items.is_empty() {
                let remaining_stream = Box::pin(futures::stream::iter(
                  all_items.into_iter().map(|item| Ok::<T, MergeError>(item)),
                ));
                let mut new_streams = Vec::new();
                new_streams.push(remaining_stream);
                input.next().await; // Consume the current streams
              }
              return Some((Ok(item), input));
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

  fn create_stream<T: Send + 'static>(
    items: Vec<T>,
  ) -> Pin<Box<dyn Stream<Item = Result<T, MergeError>> + Send>> {
    Box::pin(stream::iter(items.into_iter().map(Ok)))
  }

  // #[tokio::test]
  // async fn test_merge_basic() {
  //   let mut transformer = MergeTransformer::<i32>::new();
  //   let streams = vec![create_stream(vec![1, 2, 3]), create_stream(vec![4, 5, 6])];
  //   let input = stream::iter(vec![Ok(streams)]);
  //   let boxed_input = Box::pin(input);
  //   let result: Vec<i32> = transformer
  //     .transform(boxed_input)
  //     .try_collect()
  //     .await
  //     .unwrap();
  //   assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);
  // }

  #[tokio::test]
  async fn test_merge_empty_input() {
    let mut transformer = MergeTransformer::<i32>::new();
    let input = stream::iter(vec![Ok(Vec::new())]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_merge_empty_streams() {
    let mut transformer = MergeTransformer::<i32>::new();
    let streams = vec![
      create_stream(Vec::<i32>::new()),
      create_stream(Vec::<i32>::new()),
      create_stream(Vec::<i32>::new()),
    ];
    let input = stream::iter(vec![Ok(streams)]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_propagation() {
    let mut transformer = MergeTransformer::<i32>::new();
    let streams = vec![
      create_stream(vec![1, 2]),
      Box::pin(stream::iter(vec![
        Ok(3),
        Err(MergeError::Transform(TransformError::Custom(
          "test error".to_string(),
        ))),
        Ok(4),
      ])) as Pin<Box<dyn Stream<Item = Result<_, MergeError>> + Send>>,
    ];
    let input = stream::iter(vec![Ok(streams)]);
    let boxed_input = Box::pin(input);

    let result = transformer
      .transform(boxed_input)
      .try_collect::<Vec<_>>()
      .await;

    assert!(result.is_err());
    match result {
      Err(MergeError::Transform(e)) => assert_eq!(e.to_string(), "test error"),
      _ => panic!("Expected TransformError"),
    }
  }
}
