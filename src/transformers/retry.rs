use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::TryStreamExt;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::time::{Duration, Instant};

pub struct RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  max_retries: usize,
  backoff: Duration,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(max_retries: usize, backoff: Duration) -> Self {
    Self {
      max_retries,
      backoff,
      config: TransformerConfig::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Input for RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let max_retries = self.max_retries;
    let backoff = self.backoff;
    Box::pin(input.then(move |item| {
      let mut retries = 0;
      async move {
        loop {
          match item {
            Ok(item) => return Ok(item),
            Err(e) => {
              if retries >= max_retries {
                return Err(e);
              }
              retries += 1;
              tokio::time::sleep(backoff).await;
            }
          }
        }
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Transformer(self.component_info().name),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "retry_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::TryStreamExt;
  use futures::stream;
  use tokio::time::sleep;

  #[tokio::test]
  async fn test_retry_basic() {
    let mut transformer = RetryTransformer::new(3, Duration::from_millis(10));
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_retry_empty_input() {
    let mut transformer = RetryTransformer::new(3, Duration::from_millis(10));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_retry_with_error() {
    let mut transformer = RetryTransformer::new(3, Duration::from_millis(10));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(|x| {
      if x == 2 {
        Err(StreamError::new(
          Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
          ErrorContext {
            timestamp: chrono::Utc::now(),
            item: None,
            stage: PipelineStage::Transformer("test".to_string()),
          },
          ComponentInfo {
            name: "test".to_string(),
            type_name: "test".to_string(),
          },
        ))
      } else {
        Ok(x)
      }
    }));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![1, 3]);
  }

  #[tokio::test]
  async fn test_retry_actual_retry() {
    let mut transformer = RetryTransformer::new(3, Duration::from_millis(10));
    let mut attempts = 0;
    let input = stream::iter(vec![1, 2, 3].into_iter().map(move |x| {
      attempts += 1;
      if attempts <= 2 {
        Err(StreamError::new(
          Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
          ErrorContext {
            timestamp: chrono::Utc::now(),
            item: None,
            stage: PipelineStage::Transformer("test".to_string()),
          },
          ComponentInfo {
            name: "test".to_string(),
            type_name: "test".to_string(),
          },
        ))
      } else {
        Ok(x)
      }
    }));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = RetryTransformer::new(3, Duration::from_millis(10))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
