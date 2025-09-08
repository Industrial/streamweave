use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::transformers::timeout::TimeoutTransformer;
use crate::traits::transformer::{Transformer, TransformerConfig};
use async_stream;
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for TimeoutTransformer<T> {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let duration = self.duration;
    Box::pin(async_stream::stream! {
      let mut input = input;

      loop {
        match tokio::time::timeout(duration, input.next()).await {
          Ok(Some(item)) => yield item,
          Ok(None) => break,
          Err(_) => {
            // On timeout, stop the stream
            break;
          }
        }
      }
    })
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
        .unwrap_or_else(|| "timeout_transformer".to_string()),
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
  async fn test_timeout_basic() {
    let mut transformer = TimeoutTransformer::new(Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_timeout_empty_input() {
    let mut transformer = TimeoutTransformer::new(Duration::from_millis(100));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_timeout_actual_timeout() {
    let mut transformer = TimeoutTransformer::new(Duration::from_millis(50));
    let input = stream::iter(vec![1, 2, 3].into_iter()).then(|x| async move {
      sleep(Duration::from_millis(100)).await;
      x
    });
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = TimeoutTransformer::new(Duration::from_millis(100))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
