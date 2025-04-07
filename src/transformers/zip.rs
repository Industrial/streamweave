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
    Box::pin(input.map(|items| {
      let mut result = Vec::new();
      let mut iterators: Vec<_> = items.into_iter().map(|item| vec![item]).collect();

      while !iterators.is_empty() {
        let mut current = Vec::new();
        let mut empty_indices = Vec::new();

        for (i, iter) in iterators.iter_mut().enumerate() {
          if let Some(item) = iter.pop() {
            current.push(item);
          } else {
            empty_indices.push(i);
          }
        }

        if !current.is_empty() {
          result.push(current);
        }

        for &i in empty_indices.iter().rev() {
          iterators.remove(i);
        }
      }

      result
    }))
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
      stage: PipelineStage::Transformer(self.component_info().name),
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
  use futures::StreamExt;
  use futures::stream;

  #[tokio::test]
  async fn test_zip_basic() {
    let other = stream::iter(vec!['a', 'b', 'c'].into_iter());
    let mut transformer = ZipTransformer::new(Box::pin(other));
    let input = stream::iter(vec![1, 2, 3].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<(i32, char)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![(1, 'a'), (2, 'b'), (3, 'c')]);
  }

  #[tokio::test]
  async fn test_zip_empty_input() {
    let other = stream::iter(Vec::<char>::new());
    let mut transformer = ZipTransformer::new(Box::pin(other));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(i32, char)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<(i32, char)>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let other = stream::iter(vec!['a']);
    let mut transformer = ZipTransformer::new(Box::pin(other))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
