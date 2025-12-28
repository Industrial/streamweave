use crate::time_window_transformer::TimeWindowTransformer;
use async_trait::async_trait;
use futures::StreamExt;
use std::collections::VecDeque;
use streamweave::{Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::time::interval;

#[async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for TimeWindowTransformer<T> {
  type InputPorts = (T,);
  type OutputPorts = (Vec<T>,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let duration = self.duration;
    Box::pin(async_stream::stream! {
      let mut window: VecDeque<T> = VecDeque::new();
      let mut input = input;
      let mut interval = interval(duration);

      // Skip the first tick (interval starts immediately)
      interval.tick().await;

      loop {
        tokio::select! {
          // Check if interval has ticked (time window expired)
          _ = interval.tick() => {
            // Time window expired, emit current window if it has items
            if !window.is_empty() {
              let window_vec: Vec<T> = window.iter().cloned().collect();
              yield window_vec;
              window.clear();
            }
          }
          // Check for new items from input stream
          item = input.next() => {
            match item {
              Some(item) => {
                window.push_back(item);
              }
              None => {
                // Stream ended, emit any remaining items as partial window
                if !window.is_empty() {
                  yield window.iter().cloned().collect::<Vec<_>>();
                }
                break;
              }
            }
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
        .unwrap_or_else(|| "time_window_transformer".to_string()),
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
  async fn test_time_window_basic() {
    let mut transformer = TimeWindowTransformer::new(Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    // Should have at least one window
    assert!(!result.is_empty());
    // Should have received all items
    let total_items: usize = result.iter().map(|w| w.len()).sum();
    assert_eq!(total_items, 5);
  }

  #[tokio::test]
  async fn test_time_window_empty_input() {
    let mut transformer = TimeWindowTransformer::new(Duration::from_millis(100));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = TimeWindowTransformer::new(Duration::from_secs(1))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
