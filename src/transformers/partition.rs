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

pub struct PartitionTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(&T) -> bool + Send + 'static,
{
  predicate: F,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T, F> PartitionTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(&T) -> bool + Send + 'static,
{
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
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

impl<T, F> Input for PartitionTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(&T) -> bool + Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T, F> Output for PartitionTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(&T) -> bool + Send + 'static,
{
  type Output = (Vec<T>, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (Vec<T>, Vec<T>)> + Send>>;
}

#[async_trait]
impl<T, F> Transformer for PartitionTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(&T) -> bool + Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let predicate = &mut self.predicate;
    Box::pin(input.collect::<Vec<_>>().then(move |items| async move {
      let mut first = Vec::new();
      let mut second = Vec::new();
      for item in items {
        if predicate(&item) {
          first.push(item);
        } else {
          second.push(item);
        }
      }
      futures::stream::iter(vec![(first, second)])
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
        .unwrap_or_else(|| "partition_transformer".to_string()),
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
  async fn test_partition_basic() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: (Vec<i32>, Vec<i32>) = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.0, vec![2, 4]);
    assert_eq!(result.1, vec![1, 3, 5]);
  }

  #[tokio::test]
  async fn test_partition_empty_input() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: (Vec<i32>, Vec<i32>) = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.0, Vec::<i32>::new());
    assert_eq!(result.1, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = PartitionTransformer::new(|x: &i32| x % 2 == 0)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
