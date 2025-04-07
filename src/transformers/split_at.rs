use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::{FutureExt, Stream, StreamExt};
use std::pin::Pin;

pub struct SplitAtTransformer<T: Send + 'static + Clone> {
  index: usize,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + 'static + Clone> SplitAtTransformer<T> {
  pub fn new(index: usize) -> Self {
    Self {
      index,
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

impl<T: Send + 'static + Clone> Input for SplitAtTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T: Send + 'static + Clone> Output for SplitAtTransformer<T> {
  type Output = (Vec<T>, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (Vec<T>, Vec<T>)> + Send>>;
}

#[async_trait]
impl<T: Send + 'static + Clone> Transformer for SplitAtTransformer<T> {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let index = self.index;
    Box::pin(input.collect::<Vec<_>>().then(move |items| async move {
      let (first, second) = items.split_at(index);
      futures::stream::iter(vec![(first.to_vec(), second.to_vec())])
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
        .unwrap_or_else(|| "split_at_transformer".to_string()),
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
  async fn test_split_at_basic() {
    let mut transformer = SplitAtTransformer::new(2);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let result: (Vec<i32>, Vec<i32>) = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.0, vec![1, 2]);
    assert_eq!(result.1, vec![3, 4, 5]);
  }

  #[tokio::test]
  async fn test_split_at_empty_input() {
    let mut transformer = SplitAtTransformer::new(2);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: (Vec<i32>, Vec<i32>) = transformer.transform(boxed_input).collect().await;

    assert_eq!(result.0, Vec::<i32>::new());
    assert_eq!(result.1, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = SplitAtTransformer::new(2)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
