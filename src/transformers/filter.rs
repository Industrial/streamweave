use crate::error::TransformError;
use crate::traits::{input::Input, output::Output, transformer::Transformer};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error;
use std::fmt;
use std::pin::Pin;

pub struct FilterTransformer<F, T> {
  predicate: F,
  _phantom: std::marker::PhantomData<T>,
}

impl<F, T> FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: Send + 'static,
{
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      _phantom: std::marker::PhantomData,
    }
  }
}

#[derive(Debug)]
pub enum FilterError {
  Transform(TransformError),
  PredicateError(String),
}

impl fmt::Display for FilterError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      FilterError::Transform(e) => write!(f, "{}", e),
      FilterError::PredicateError(msg) => write!(f, "Filter predicate failed: {}", msg),
    }
  }
}

impl Error for FilterError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      FilterError::Transform(e) => Some(e),
      _ => None,
    }
  }
}

impl<F, T: Send + 'static> crate::traits::error::Error for FilterTransformer<F, T> {
  type Error = FilterError;
}

impl<F, T: Send + 'static> Input for FilterTransformer<F, T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<F, T: Send + 'static> Output for FilterTransformer<F, T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
}

#[async_trait]
impl<F, T> Transformer for FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: Send + Clone + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let mut predicate = self.predicate.clone();

    Box::pin(futures::stream::unfold(input, move |mut input| {
      let mut predicate = predicate.clone();
      async move {
        while let Some(result) = input.next().await {
          match result {
            Ok(item) => {
              if predicate(&item) {
                return Some((Ok(item), input));
              }
              // Continue to next item if predicate returns false
              continue;
            }
            Err(e) => return Some((Err(e), input)),
          }
        }
        None
      }
    }))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::TryStreamExt;
  use futures::stream;

  #[tokio::test]
  async fn test_filter_even_numbers() {
    let mut transformer = FilterTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).try_collect().await.unwrap();

    assert_eq!(result, vec![2, 4, 6]);
  }

  #[tokio::test]
  async fn test_filter_empty_input() {
    let mut transformer = FilterTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(Vec::<Result<i32, FilterError>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).try_collect().await.unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_filter_all_match() {
    let mut transformer = FilterTransformer::new(|x: &i32| *x > 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).try_collect().await.unwrap();

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_filter_none_match() {
    let mut transformer = FilterTransformer::new(|x: &i32| *x > 10);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).try_collect().await.unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_filter_with_strings() {
    let mut transformer = FilterTransformer::new(|s: &String| s.starts_with("a"));
    let input = stream::iter(
      vec![
        "apple".to_string(),
        "banana".to_string(),
        "avocado".to_string(),
        "cherry".to_string(),
      ]
      .into_iter()
      .map(Ok),
    );
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer.transform(boxed_input).try_collect().await.unwrap();

    assert_eq!(result, vec!["apple".to_string(), "avocado".to_string()]);
  }

  #[tokio::test]
  async fn test_error_propagation() {
    let mut transformer = FilterTransformer::new(|_: &i32| true);
    let input = stream::iter(vec![
      Ok(1),
      Ok(2),
      Err(FilterError::Transform(TransformError::Custom(
        "test error".to_string(),
      ))),
      Ok(4),
    ]);
    let boxed_input = Box::pin(input);

    let result = transformer.transform(boxed_input).try_collect::<Vec<_>>().await;

    assert!(result.is_err());
    match result {
      Err(FilterError::Transform(e)) => assert_eq!(e.to_string(), "test error"),
      _ => panic!("Expected TransformError"),
    }
  }
}
