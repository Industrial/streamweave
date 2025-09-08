use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::transformers::debounce::DebounceTransformer;
use crate::traits::transformer::{Transformer, TransformerConfig};
use async_stream;
use async_trait::async_trait;
use futures::StreamExt;
use tokio::time;

#[async_trait]
impl<T> Transformer for DebounceTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let duration = self.duration;

    Box::pin(async_stream::stream! {
        let mut last_item: Option<T> = None;
        let delay = time::sleep(duration);
        tokio::pin!(delay);

        let mut input = input;

        loop {
            tokio::select! {
                maybe_item = input.next() => {
                    match maybe_item {
                        Some(item) => {
                            last_item = Some(item);
                            delay.as_mut().reset(time::Instant::now() + duration);
                        }
                        None => {
                            if let Some(item) = last_item.take() {
                                yield item;
                            }
                            break;
                        }
                    }
                }
                _ = &mut delay => {
                    if let Some(item) = last_item.take() {
                        yield item;
                    }
                    delay.as_mut().reset(time::Instant::now() + duration);
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
        .unwrap_or_else(|| "debounce_transformer".to_string()),
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

  #[tokio::test]
  async fn test_debounce_basic() {
    let mut transformer = DebounceTransformer::new(Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![5]);
  }

  #[tokio::test]
  async fn test_debounce_empty_input() {
    let mut transformer = DebounceTransformer::new(Duration::from_millis(100));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_debounce_timing() {
    let mut transformer = DebounceTransformer::new(Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![5]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = DebounceTransformer::new(Duration::from_millis(100))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
