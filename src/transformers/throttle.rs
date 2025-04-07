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
use std::time::Duration;
use tokio::time::Instant;

pub struct ThrottleTransformer<T: Send + 'static + Clone> {
  duration: Duration,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + 'static + Clone> ThrottleTransformer<T> {
  pub fn new(duration: Duration) -> Self {
    Self {
      duration,
      config: TransformerConfig::<T>::default(),
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

impl<T: Send + 'static + Clone> crate::traits::error::Error for ThrottleTransformer<T> {
  type Error = StreamError<T>;
}

impl<T: Send + 'static + Clone> Input for ThrottleTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError<T>>> + Send>>;
}

impl<T: Send + 'static + Clone> Output for ThrottleTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError<T>>> + Send>>;
}

#[async_trait]
impl<T: Send + 'static + Clone> Transformer for ThrottleTransformer<T> {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let duration = self.duration;
    Box::pin(input.throttle(duration))
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
    match self.config.error_strategy() {
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
        .name()
        .unwrap_or_else(|| "throttle_transformer".to_string()),
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
  async fn test_throttle_basic() {
    let mut transformer = ThrottleTransformer::new(Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_throttle_empty_input() {
    let mut transformer = ThrottleTransformer::new(Duration::from_millis(100));
    let input = stream::iter(Vec::<Result<i32, StreamError<i32>>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_throttle_with_error() {
    let mut transformer = ThrottleTransformer::new(Duration::from_millis(100));
    let input = stream::iter(vec![
      Ok(1),
      Err(StreamError {
        source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Transformer("test".to_string()),
        },
        retries: 0,
        component: ComponentInfo {
          name: "test".to_string(),
          type_name: "test".to_string(),
        },
      }),
      Ok(2),
    ]);
    let boxed_input = Box::pin(input);

    let result: Result<Vec<i32>, _> = transformer.transform(boxed_input).try_collect().await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_throttle_timing() {
    let mut transformer = ThrottleTransformer::new(Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter().map(|x| {
      let x = x.clone();
      async move {
        sleep(Duration::from_millis(50)).await;
        Ok(x)
      }
    }));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 3, 5]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = ThrottleTransformer::new(Duration::from_millis(100))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
