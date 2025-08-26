use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::collections::VecDeque;
use std::pin::Pin;

pub struct WindowTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  size: usize,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> WindowTransformer<T> {
  pub fn new(size: usize) -> Self {
    Self {
      size,
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

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for WindowTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for WindowTransformer<T> {
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

#[async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for WindowTransformer<T> {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let size = self.size;
    Box::pin(async_stream::stream! {
      let mut window: VecDeque<T> = VecDeque::with_capacity(size);
      let mut input = input;

      while let Some(item) = input.next().await {
        window.push_back(item);

        if window.len() == size {
          yield window.iter().cloned().collect::<Vec<_>>();
          window.clear(); // Clear the entire window instead of just popping one item
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

    assert_eq!(result, vec![vec![1, 2, 3], vec![4, 5, 6], vec![7],]);
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
    let mut transformer = WindowTransformer::new(3)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
