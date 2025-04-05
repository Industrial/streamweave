use crate::error::TransformError;
use crate::traits::{input::Input, output::Output, transformer::Transformer};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error;
use std::fmt;
use std::pin::Pin;

pub struct MapTransformer<F, I, O> {
  f: F,
  _phantom_i: std::marker::PhantomData<I>,
  _phantom_o: std::marker::PhantomData<O>,
}

impl<F, I, O> MapTransformer<F, I, O>
where
  F: FnMut(I) -> Result<O, Box<dyn Error + Send + Sync>> + Send + Clone + 'static,
  I: Send + 'static,
  O: Send + 'static,
{
  pub fn new(f: F) -> Self {
    Self {
      f,
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
    }
  }
}

#[derive(Debug)]
pub enum MapError {
  Transform(TransformError),
  MapFunctionError(String),
}

impl fmt::Display for MapError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      MapError::Transform(e) => write!(f, "{}", e),
      MapError::MapFunctionError(msg) => write!(f, "Map function failed: {}", msg),
    }
  }
}

impl Error for MapError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      MapError::Transform(e) => Some(e),
      _ => None,
    }
  }
}

impl<F, I, O> crate::traits::error::Error for MapTransformer<F, I, O>
where
  F: Send + 'static,
  I: Send + 'static,
  O: Send + 'static,
{
  type Error = MapError;
}

impl<F, I, O> Input for MapTransformer<F, I, O>
where
  F: Send + 'static,
  I: Send + 'static,
  O: Send + 'static,
{
  type Input = I;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<F, I, O> Output for MapTransformer<F, I, O>
where
  F: Send + 'static,
  I: Send + 'static,
  O: Send + 'static,
{
  type Output = O;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
}

#[async_trait]
impl<F, I, O> Transformer for MapTransformer<F, I, O>
where
  F: FnMut(I) -> Result<O, Box<dyn Error + Send + Sync>> + Send + Clone + 'static,
  I: Send + 'static,
  O: Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let f = self.f.clone();

    Box::pin(futures::stream::unfold(
      (input, f),
      |(mut input, mut f)| async move {
        match input.next().await {
          Some(result) => {
            let mapped = match result {
              Ok(item) => match f(item) {
                Ok(output) => Ok(output),
                Err(e) => Err(MapError::Transform(TransformError::OperationFailed(e))),
              },
              Err(e) => Err(e),
            };
            Some((mapped, (input, f)))
          }
          None => None,
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
  async fn test_map_transformer() {
    let mut transformer = MapTransformer::new(|x: i32| Ok(x * 2));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![2, 4, 6]);
  }

  #[tokio::test]
  async fn test_map_transformer_type_conversion() {
    let mut transformer = MapTransformer::new(|x: i32| Ok(x.to_string()));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec!["1", "2", "3"]);
  }

  #[tokio::test]
  async fn test_map_transformer_with_error() {
    let mut transformer =
      MapTransformer::new(|x: i32| -> Result<i32, Box<dyn Error + Send + Sync>> {
        if x % 2 == 0 {
          Ok(x * 2)
        } else {
          Err("Odd numbers not allowed".into())
        }
      });

    let input = stream::iter(vec![1, 2, 3, 4].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result = transformer
      .transform(boxed_input)
      .try_collect::<Vec<_>>()
      .await;

    assert!(result.is_err());
    match result {
      Err(MapError::MapFunctionError(e)) => assert_eq!(e, "Odd numbers not allowed"),
      _ => panic!("Expected MapFunctionError"),
    }
  }

  #[tokio::test]
  async fn test_map_transformer_reuse() {
    let mut transformer = MapTransformer::new(|x: i32| Ok(x * 2));

    // First transform
    let input1 = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input1 = Box::pin(input1);
    let result1: Vec<i32> = transformer
      .transform(boxed_input1)
      .try_collect()
      .await
      .unwrap();
    assert_eq!(result1, vec![2, 4, 6]);

    // Second transform
    let input2 = stream::iter(vec![4, 5, 6].into_iter().map(Ok));
    let boxed_input2 = Box::pin(input2);
    let result2: Vec<i32> = transformer
      .transform(boxed_input2)
      .try_collect()
      .await
      .unwrap();
    assert_eq!(result2, vec![8, 10, 12]);
  }
}
