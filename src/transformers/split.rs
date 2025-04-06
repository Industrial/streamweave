use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::stream;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct SplitTransformer<F, T> {
  predicate: F,
  _phantom: std::marker::PhantomData<T>,
  config: TransformerConfig,
}

impl<F, T> SplitTransformer<F, T>
where
  F: FnMut(&T) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> + Send + Clone + 'static,
  T: Clone + Send + 'static,
{
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      _phantom: std::marker::PhantomData,
      config: TransformerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    self.config_mut().set_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config_mut().set_name(name);
    self
  }
}

impl<F, T> crate::traits::error::Error for SplitTransformer<F, T>
where
  F: Send + 'static,
  T: Send + 'static,
{
  type Error = StreamError;
}

impl<F, T> Input for SplitTransformer<F, T>
where
  F: Send + 'static,
  T: Send + 'static,
{
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<F, T> Output for SplitTransformer<F, T>
where
  F: Send + 'static,
  T: Send + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<F, T> Transformer for SplitTransformer<F, T>
where
  F: FnMut(&T) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> + Send + Clone + 'static,
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
                    let error =
                      StreamError::new(e, self.create_error_context(None), self.component_info());
                    return Some((Err(error), (input, pred)));
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
                      as Pin<Box<dyn Stream<Item = Result<Vec<T>, StreamError>> + Send>>,
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

  fn config(&self) -> &TransformerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut TransformerConfig {
    &mut self.config
  }

  fn handle_error(&self, error: StreamError) -> ErrorStrategy {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorStrategy::Retry(n),
      _ => ErrorStrategy::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Transformer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "split_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
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
  async fn test_error_handling_strategies() {
    let mut transformer = SplitTransformer::new(|_: &i32| Ok(true))
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      transformer.create_error_context(None),
      transformer.component_info(),
    );

    assert_eq!(transformer.handle_error(error), ErrorStrategy::Skip);
  }
}
