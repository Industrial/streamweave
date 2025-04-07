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

pub struct SortTransformer<T>
where
  T: Clone + Send + 'static,
{
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> SortTransformer<T>
where
  T: Clone + Send + 'static,
{
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
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

impl<T> Input for SortTransformer<T>
where
  T: Clone + Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for SortTransformer<T>
where
  T: Clone + Send + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for SortTransformer<T>
where
  T: Clone + Send + 'static + Ord,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.collect::<Vec<_>>().then(|items| async move {
      let mut items = items;
      items.sort();
      futures::stream::iter(items.into_iter())
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
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
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
        .name()
        .unwrap_or_else(|| "sort_transformer".to_string()),
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
  async fn test_sort_basic() {
    let mut transformer = SortTransformer::new();
    let input = stream::iter(vec![3, 1, 4, 1, 5, 9]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 1, 3, 4, 5, 9]);
  }

  #[tokio::test]
  async fn test_sort_empty_input() {
    let mut transformer = SortTransformer::new();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_sort_with_error() {
    let mut transformer = SortTransformer::new();
    let input = stream::iter(vec![3, 1, 4].into_iter().map(|x| {
      if x == 1 {
        Err(StreamError::new(
          Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
          ErrorContext {
            timestamp: chrono::Utc::now(),
            item: None,
            stage: PipelineStage::Transformer("test".to_string()),
          },
          ComponentInfo {
            name: "test".to_string(),
            type_name: "test".to_string(),
          },
        ))
      } else {
        Ok(x)
      }
    }));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert_eq!(result, vec![3, 4]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = SortTransformer::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
