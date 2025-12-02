use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::window::window_transformer::WindowTransformer;
use async_trait::async_trait;
use futures::StreamExt;
use std::collections::VecDeque;

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
}
