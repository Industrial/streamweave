use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::transformers::rate_limit::RateLimitTransformer;
use crate::traits::transformer::{Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::time::Instant;

#[async_trait]
impl<T> Transformer for RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let rate_limit = self.rate_limit;
    let time_window = self.time_window;
    let count = Arc::clone(&self.count);
    let window_start = Arc::clone(&self.window_start);

    Box::pin(
      input
        .then(move |item| {
          let count = Arc::clone(&count);
          let window_start = Arc::clone(&window_start);
          async move {
            let now = Instant::now();
            let mut window_start = window_start.write().await;

            if now.duration_since(*window_start) > time_window {
              count.store(0, Ordering::SeqCst);
              *window_start = now;
            }

            if count.load(Ordering::SeqCst) >= rate_limit {
              None
            } else {
              count.fetch_add(1, Ordering::SeqCst);
              Some(item)
            }
          }
        })
        .filter_map(futures::future::ready),
    )
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
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
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
        .name
        .clone()
        .unwrap_or_else(|| "rate_limit_transformer".to_string()),
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
  use tokio::time::sleep;

  #[tokio::test]
  async fn test_rate_limit_basic() {
    let mut transformer = RateLimitTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_rate_limit_empty_input() {
    let mut transformer = RateLimitTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_rate_limit_actual_rate_limit() {
    let mut transformer = RateLimitTransformer::new(2, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2]);
  }

  #[tokio::test]
  async fn test_rate_limit_reset() {
    let mut transformer = RateLimitTransformer::new(2, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2]);

    sleep(Duration::from_millis(200)).await;

    let input = stream::iter(vec![3, 4]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![3, 4]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = RateLimitTransformer::new(3, Duration::from_millis(100))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
