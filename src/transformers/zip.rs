use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct ZipTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  config: TransformerConfig<Vec<T>>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> ZipTransformer<T> {
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::<Vec<T>>::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Vec<T>>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for ZipTransformer<T> {
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for ZipTransformer<T> {
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

#[async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Transformer for ZipTransformer<T> {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(async_stream::stream! {
      let mut input = input;
      let mut buffers: Vec<Vec<T>> = Vec::new();

      while let Some(items) = input.next().await {
        buffers.push(items);
      }

      // Early return if no input
      if buffers.is_empty() {
        return;
      }

      // Get the length of the longest vector
      let max_len = buffers.iter().map(|v| v.len()).max().unwrap_or(0);

      // Yield transposed vectors
      for i in 0..max_len {
        let mut result = Vec::new();
        for buffer in &buffers {
          if let Some(item) = buffer.get(i) {
            result.push(item.clone());
          }
        }
        if !result.is_empty() {
          yield result;
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Vec<T>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Vec<T>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Vec<T>> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Vec<T>>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Vec<T>>) -> ErrorContext<Vec<T>> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "zip_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "zip_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_zip_basic() {
    let mut transformer = ZipTransformer::new();
    let input = stream::iter(vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]]);
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, 4, 7], vec![2, 5, 8], vec![3, 6, 9]]);
  }

  #[tokio::test]
  async fn test_zip_empty_input() {
    let mut transformer = ZipTransformer::new();
    let input = stream::iter(Vec::<Vec<i32>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = ZipTransformer::new()
      .with_error_strategy(ErrorStrategy::<Vec<i32>>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.get_config_impl();
    assert_eq!(config.error_strategy, ErrorStrategy::<Vec<i32>>::Skip);
    assert_eq!(config.name, Some("test_transformer".to_string()));
  }
}
