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

pub struct InterleaveTransformer<T> {
  other: Pin<Box<dyn Stream<Item = Result<T, StreamError>> + Send>>,
  config: TransformerConfig,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> InterleaveTransformer<T>
where
  T: Send + 'static,
{
  pub fn new(other: Pin<Box<dyn Stream<Item = Result<T, StreamError>> + Send>>) -> Self {
    Self {
      other,
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

impl<T> crate::traits::error::Error for InterleaveTransformer<T>
where
  T: Send + 'static,
{
  type Error = StreamError;
}

impl<T> Input for InterleaveTransformer<T>
where
  T: Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<T> Output for InterleaveTransformer<T>
where
  T: Send + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<T> Transformer for InterleaveTransformer<T>
where
  T: Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let other = self.other.clone();
    Box::pin(input.interleave(other))
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
        .unwrap_or_else(|| "interleave_transformer".to_string()),
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
  async fn test_interleave_basic() {
    let other = stream::iter(vec![4, 5, 6].into_iter().map(Ok));
    let mut transformer = InterleaveTransformer::new(Box::pin(other));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 4, 2, 5, 3, 6]);
  }

  #[tokio::test]
  async fn test_interleave_empty_input() {
    let other = stream::iter(Vec::<Result<i32, StreamError>>::new());
    let mut transformer = InterleaveTransformer::new(Box::pin(other));
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
  async fn test_interleave_with_error() {
    let other = stream::iter(vec![Ok(4), Ok(5)]);
    let mut transformer = InterleaveTransformer::new(Box::pin(other));
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
      Ok(2),
    ]);
    let boxed_input = Box::pin(input);

    let result: Result<Vec<i32>, _> = transformer.transform(boxed_input).try_collect().await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let other = stream::iter(vec![Ok(4)]);
    let mut transformer = InterleaveTransformer::new(Box::pin(other))
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
