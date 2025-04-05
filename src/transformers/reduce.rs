use crate::error::TransformError;
use crate::traits::{input::Input, output::Output, transformer::Transformer};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::error::Error;
use std::fmt;
use std::pin::Pin;

pub struct ReduceTransformer<F, T, Acc> {
  reducer: F,
  initial: Acc,
  _phantom: std::marker::PhantomData<T>,
}

impl<F, T, Acc> ReduceTransformer<F, T, Acc>
where
  F: FnMut(Acc, T) -> Result<Acc, Box<dyn Error + Send + Sync>> + Send + Clone + 'static,
  T: Send + 'static,
  Acc: Send + Clone + 'static,
{
  pub fn new(reducer: F, initial: Acc) -> Self {
    Self {
      reducer,
      initial,
      _phantom: std::marker::PhantomData,
    }
  }
}

#[derive(Debug)]
pub enum ReduceError {
  Transform(TransformError),
  InitialValueMissing,
  ReductionFailed,
}

impl fmt::Display for ReduceError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ReduceError::Transform(e) => write!(f, "{}", e),
      ReduceError::InitialValueMissing => write!(f, "Initial value required for reduction"),
      ReduceError::ReductionFailed => write!(f, "Failed to apply reduction operation"),
    }
  }
}

impl Error for ReduceError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    match self {
      ReduceError::Transform(e) => Some(e),
      _ => None,
    }
  }
}

impl<F, T, Acc> crate::traits::error::Error for ReduceTransformer<F, T, Acc>
where
  F: Send + 'static,
  T: Send + 'static,
  Acc: Send + 'static,
{
  type Error = ReduceError;
}

impl<F, T, Acc> Input for ReduceTransformer<F, T, Acc>
where
  F: Send + 'static,
  T: Send + 'static,
  Acc: Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<F, T, Acc> Output for ReduceTransformer<F, T, Acc>
where
  F: Send + 'static,
  T: Send + 'static,
  Acc: Send + 'static,
{
  type Output = Acc;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
}

#[async_trait]
impl<F, T, Acc> Transformer for ReduceTransformer<F, T, Acc>
where
  F: FnMut(Acc, T) -> Result<Acc, Box<dyn Error + Send + Sync>> + Send + Clone + 'static,
  T: Send + 'static,
  Acc: Send + Clone + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let reducer = self.reducer.clone();
    let initial = self.initial.clone();

    Box::pin(futures::stream::unfold(
      (input, Some(initial), reducer),
      |(mut input, mut acc, mut reducer)| async move {
        while let Some(item_result) = input.next().await {
          match item_result {
            Ok(item) => {
              if let Some(current_acc) = acc.take() {
                match reducer(current_acc, item) {
                  Ok(new_acc) => {
                    acc = Some(new_acc.clone());
                    return Some((Ok(new_acc), (input, acc, reducer)));
                  }
                  Err(e) => {
                    return Some((
                      Err(ReduceError::Transform(TransformError::OperationFailed(e))),
                      (input, None, reducer),
                    ));
                  }
                }
              }
            }
            Err(e) => return Some((Err(e), (input, acc, reducer))),
          }
        }
        None
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
  async fn test_reduce_transformer_sum() {
    let mut transformer = ReduceTransformer::new(|acc, x| Ok(acc + x), 0);
    let input = vec![1, 2, 3, 4, 5];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<i32> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 3, 6, 10, 15]);
  }

  #[tokio::test]
  async fn test_reduce_transformer_string_concat() {
    let mut transformer = ReduceTransformer::new(|acc: String, x: &str| Ok(acc + x), String::new());
    let input = vec!["a", "b", "c"];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<String> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(
      result,
      vec!["a".to_string(), "ab".to_string(), "abc".to_string()]
    );
  }

  #[tokio::test]
  async fn test_reduce_transformer_with_error() {
    let mut transformer = ReduceTransformer::new(
      |acc: i32, x: i32| {
        if x % 2 == 0 {
          Ok(acc + x)
        } else {
          Err("Odd numbers not allowed".into())
        }
      },
      0,
    );

    let input = vec![2, 3, 4, 5, 6];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result = transformer
      .transform(input_stream)
      .try_collect::<Vec<_>>()
      .await;

    assert!(result.is_err());
    match result.unwrap_err() {
      ReduceError::Transform(TransformError::OperationFailed(e)) => {
        assert_eq!(e.to_string(), "Odd numbers not allowed")
      }
      _ => panic!("Expected OperationFailed error"),
    }
  }

  #[tokio::test]
  async fn test_reduce_transformer_custom_type() {
    #[derive(Debug, Clone, PartialEq)]
    struct Counter {
      count: i32,
      sum: i32,
    }

    let mut transformer = ReduceTransformer::new(
      |acc: Counter, x: i32| {
        Ok(Counter {
          count: acc.count + 1,
          sum: acc.sum + x,
        })
      },
      Counter { count: 0, sum: 0 },
    );

    let input = vec![1, 2, 3];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<Counter> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(
      result,
      vec![
        Counter { count: 1, sum: 1 },
        Counter { count: 2, sum: 3 },
        Counter { count: 3, sum: 6 },
      ]
    );
  }
}
