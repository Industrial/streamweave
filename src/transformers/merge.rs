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

pub struct MergeTransformer<T>
where
  T: Send + 'static + Clone,
{
  _phantom: std::marker::PhantomData<T>,
  config: TransformerConfig<Vec<Pin<Box<dyn Stream<Item = T> + Send>>>>,
}

impl<T> MergeTransformer<T>
where
  T: Send + 'static + Clone,
{
  pub fn new() -> Self {
    Self {
      _phantom: std::marker::PhantomData,
      config: TransformerConfig::default(),
    }
  }

  pub fn with_error_strategy(
    mut self,
    strategy: ErrorStrategy<Vec<Pin<Box<dyn Stream<Item = T> + Send>>>>,
  ) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Default for MergeTransformer<T>
where
  T: Send + 'static + Clone,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Input for MergeTransformer<T>
where
  T: Send + 'static + Clone,
{
  type Input = Vec<Pin<Box<dyn Stream<Item = T> + Send>>>;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl<T> Output for MergeTransformer<T>
where
  T: Send + 'static + Clone,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for MergeTransformer<T>
where
  T: Send + 'static + Clone,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.flat_map(|streams| futures::stream::select_all(streams)))
  }

  fn set_config_impl(
    &mut self,
    config: TransformerConfig<Vec<Pin<Box<dyn Stream<Item = T> + Send>>>>,
  ) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Vec<Pin<Box<dyn Stream<Item = T> + Send>>>> {
    &self.config
  }

  fn get_config_mut_impl(
    &mut self,
  ) -> &mut TransformerConfig<Vec<Pin<Box<dyn Stream<Item = T> + Send>>>> {
    &mut self.config
  }

  fn handle_error(
    &self,
    error: &StreamError<Vec<Pin<Box<dyn Stream<Item = T> + Send>>>>,
  ) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(
    &self,
    item: Option<Vec<Pin<Box<dyn Stream<Item = T> + Send>>>>,
  ) -> ErrorContext<Vec<Pin<Box<dyn Stream<Item = T> + Send>>>> {
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
        .unwrap_or_else(|| "merge_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use futures::stream;

  fn create_stream<T: Send + 'static>(items: Vec<T>) -> Pin<Box<dyn Stream<Item = T> + Send>> {
    Box::pin(stream::iter(items))
  }

  #[tokio::test]
  async fn test_merge_empty_input() {
    let mut transformer = MergeTransformer::<i32>::new();
    let input = stream::iter(vec![Vec::new()]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_merge_empty_streams() {
    let mut transformer = MergeTransformer::<i32>::new();
    let streams = vec![
      create_stream(Vec::<i32>::new()),
      create_stream(Vec::<i32>::new()),
      create_stream(Vec::<i32>::new()),
    ];
    let input = stream::iter(vec![streams]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = MergeTransformer::<i32>::new()
      .with_error_strategy(ErrorStrategy::<Vec<Pin<Box<dyn Stream<Item = i32> + Send>>>>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(
      config.error_strategy(),
      ErrorStrategy::<Vec<Pin<Box<dyn Stream<Item = i32> + Send>>>>::Skip
    );
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
