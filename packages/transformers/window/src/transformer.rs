use crate::window_transformer::WindowTransformer;
use async_trait::async_trait;
use futures::StreamExt;
use std::collections::VecDeque;
use streamweave_core::{Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for WindowTransformer<T> {
  type InputPorts = (T,);
  type OutputPorts = (Vec<T>,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let size = self.size;
    Box::pin(async_stream::stream! {
      let mut window: VecDeque<T> = VecDeque::with_capacity(size);
      let mut input = input;

      while let Some(item) = input.next().await {
        window.push_back(item);

        if window.len() == size {
          // Efficiently yield the current window as a Vec
          let window_vec: Vec<T> = window.iter().cloned().collect();
          yield window_vec;
          // Slide the window by removing the oldest item
          window.pop_front();
        }
      }

      // Emit any remaining items as a partial window
      if !window.is_empty() {
        yield window.iter().cloned().collect::<Vec<_>>();
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
        .unwrap_or_else(|| "window_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use futures::stream;

  #[tokio::test]
  async fn test_window_basic() {
    let mut transformer = WindowTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    // Sliding window of size 3: each window overlaps by 2 items
    // The transformer also emits a partial window at the end if there are remaining items
    assert_eq!(
      result,
      vec![
        vec![1, 2, 3],
        vec![2, 3, 4],
        vec![3, 4, 5],
        vec![4, 5, 6],
        vec![5, 6, 7],
        vec![6, 7] // Partial window at the end
      ]
    );
  }

  #[tokio::test]
  async fn test_window_empty_input() {
    let mut transformer = WindowTransformer::new(3);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = WindowTransformer::new(3)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }

  #[tokio::test]
  async fn test_window_size_one() {
    let mut transformer = WindowTransformer::new(1);
    let input = stream::iter(vec![1, 2, 3].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1], vec![2], vec![3]]);
  }

  #[tokio::test]
  async fn test_window_larger_than_input() {
    let mut transformer = WindowTransformer::new(10);
    let input = stream::iter(vec![1, 2, 3].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    // Should emit partial window at the end
    assert_eq!(result, vec![vec![1, 2, 3]]);
  }

  #[tokio::test]
  async fn test_set_config_impl() {
    let mut transformer = WindowTransformer::<i32>::new(3);
    let new_config = TransformerConfig::<i32> {
      name: Some("test_transformer".to_string()),
      error_strategy: ErrorStrategy::<i32>::Skip,
    };

    transformer.set_config_impl(new_config.clone());
    assert_eq!(transformer.get_config_impl().name, new_config.name);
    assert_eq!(
      transformer.get_config_impl().error_strategy,
      new_config.error_strategy
    );
  }

  #[tokio::test]
  async fn test_get_config_mut_impl() {
    let mut transformer = WindowTransformer::<i32>::new(3);
    let config_mut = transformer.get_config_mut_impl();
    config_mut.name = Some("mutated_name".to_string());
    assert_eq!(
      transformer.get_config_impl().name,
      Some("mutated_name".to_string())
    );
  }

  #[tokio::test]
  async fn test_handle_error_stop() {
    let transformer = WindowTransformer::new(3).with_error_strategy(ErrorStrategy::<i32>::Stop);
    let error = StreamError::new(
      Box::new(std::io::Error::other("test error")),
      ErrorContext::default(),
      ComponentInfo::default(),
    );
    assert_eq!(transformer.handle_error(&error), ErrorAction::Stop);
  }

  #[tokio::test]
  async fn test_handle_error_skip() {
    let transformer = WindowTransformer::new(3).with_error_strategy(ErrorStrategy::<i32>::Skip);
    let error = StreamError::new(
      Box::new(std::io::Error::other("test error")),
      ErrorContext::default(),
      ComponentInfo::default(),
    );
    assert_eq!(transformer.handle_error(&error), ErrorAction::Skip);
  }

  #[tokio::test]
  async fn test_handle_error_retry_within_limit() {
    let transformer = WindowTransformer::new(3).with_error_strategy(ErrorStrategy::<i32>::Retry(5));
    let mut error = StreamError::new(
      Box::new(std::io::Error::other("test error")),
      ErrorContext::default(),
      ComponentInfo::default(),
    );
    error.retries = 3;
    assert_eq!(transformer.handle_error(&error), ErrorAction::Retry);
  }

  #[tokio::test]
  async fn test_handle_error_retry_exceeds_limit() {
    let transformer = WindowTransformer::new(3).with_error_strategy(ErrorStrategy::<i32>::Retry(5));
    let mut error = StreamError::new(
      Box::new(std::io::Error::other("test error")),
      ErrorContext::default(),
      ComponentInfo::default(),
    );
    error.retries = 5;
    assert_eq!(transformer.handle_error(&error), ErrorAction::Stop);
  }

  #[tokio::test]
  async fn test_create_error_context() {
    let transformer = WindowTransformer::new(3).with_name("test_transformer".to_string());
    let context = transformer.create_error_context(Some(42));
    assert_eq!(context.item, Some(42));
    assert_eq!(context.component_name, "test_transformer");
    assert!(context.timestamp <= chrono::Utc::now());
  }

  #[tokio::test]
  async fn test_create_error_context_no_item() {
    let transformer = WindowTransformer::<i32>::new(3).with_name("test_transformer".to_string());
    let context = transformer.create_error_context(None);
    assert_eq!(context.item, None);
    assert_eq!(context.component_name, "test_transformer");
  }

  #[tokio::test]
  async fn test_component_info() {
    let transformer = WindowTransformer::<i32>::new(3).with_name("test_transformer".to_string());
    let info = transformer.component_info();
    assert_eq!(info.name, "test_transformer");
    assert_eq!(
      info.type_name,
      std::any::type_name::<WindowTransformer<i32>>()
    );
  }

  #[tokio::test]
  async fn test_component_info_default_name() {
    let transformer = WindowTransformer::<i32>::new(3);
    let info = transformer.component_info();
    assert_eq!(info.name, "window_transformer");
    assert_eq!(
      info.type_name,
      std::any::type_name::<WindowTransformer<i32>>()
    );
  }
}
