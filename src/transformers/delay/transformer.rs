use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::delay::delay_transformer::DelayTransformer;
use async_trait::async_trait;
use futures::StreamExt;
use tokio::time::sleep;

#[async_trait]
impl<T> Transformer for DelayTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let duration = self.duration;
    Box::pin(input.then(move |item| async move {
      sleep(duration).await;
      item
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
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "delay_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use futures::stream;
  use std::time::Duration;
  use tokio::time::Instant;

  #[tokio::test]
  async fn test_delay_basic() {
    let mut transformer = DelayTransformer::new(Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let start = Instant::now();
    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    let duration = start.elapsed();

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
    assert!(duration >= Duration::from_millis(500)); // Each item delayed by 100ms
  }

  #[tokio::test]
  async fn test_delay_empty_input() {
    let mut transformer = DelayTransformer::new(Duration::from_millis(100));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = DelayTransformer::new(Duration::from_millis(100))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
