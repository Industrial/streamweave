use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tokio::time::{Duration, Instant};

pub struct TimeoutTransformer<T> {
  duration: Duration,
  config: TransformerConfig,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> TimeoutTransformer<T>
where
  T: Send + 'static,
{
  pub fn new(duration: Duration) -> Self {
    Self {
      duration,
      config: TransformerConfig::default(),
      _phantom: std::marker::PhantomData,
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

impl<T> crate::traits::error::Error for TimeoutTransformer<T>
where
  T: Send + 'static,
{
  type Error = StreamError;
}

impl<T> Input for TimeoutTransformer<T>
where
  T: Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<T> Output for TimeoutTransformer<T>
where
  T: Send + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<T> Transformer for TimeoutTransformer<T>
where
  T: Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let duration = self.duration;
    Box::pin(input.timeout(duration).map(|result| match result {
      Ok(Ok(item)) => Ok(item),
      Ok(Err(e)) => Err(e),
      Err(_) => Err(StreamError::new(
        Box::new(std::io::Error::new(
          std::io::ErrorKind::TimedOut,
          "Operation timed out",
        )),
        self.create_error_context(None),
        self.component_info(),
      )),
    }))
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
        .unwrap_or_else(|| "timeout_transformer".to_string()),
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
  async fn test_timeout_basic() {
    let mut transformer = TimeoutTransformer::new(Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_timeout_empty_input() {
    let mut transformer = TimeoutTransformer::new(Duration::from_millis(100));
    let input = stream::iter(Vec::<Result<i32, StreamError>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_timeout_with_error() {
    let mut transformer = TimeoutTransformer::new(Duration::from_millis(100));
    let input = stream::iter(vec![
      Ok(1),
      Err(StreamError::new(
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
        ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Transformer,
        },
        ComponentInfo {
          name: "test".to_string(),
          type_name: "test".to_string(),
        },
      )),
      Ok(3),
    ]);
    let boxed_input = Box::pin(input);

    let result: Result<Vec<i32>, _> = transformer.transform(boxed_input).try_collect().await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_timeout_actual_timeout() {
    let mut transformer = TimeoutTransformer::new(Duration::from_millis(50));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok)).then(|x| async move {
      sleep(Duration::from_millis(100)).await;
      x
    });
    let boxed_input = Box::pin(input);

    let result: Result<Vec<i32>, _> = transformer.transform(boxed_input).try_collect().await;

    assert!(result.is_err());
    if let Err(e) = result {
      assert_eq!(e.source.to_string(), "Operation timed out");
    }
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = TimeoutTransformer::new(Duration::from_millis(100))
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
