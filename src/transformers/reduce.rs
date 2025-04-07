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

pub struct ReduceTransformer<T: Send + 'static + Clone, Acc: Send + 'static + Clone, F> {
  accumulator: Acc,
  reducer: F,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + 'static + Clone, Acc: Send + 'static + Clone, F> ReduceTransformer<T, Acc, F>
where
  F: FnMut(Acc, T) -> Acc + Send + 'static + Clone,
{
  pub fn new(initial: Acc, reducer: F) -> Self {
    Self {
      accumulator: initial,
      reducer,
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

impl<T: Send + 'static + Clone, Acc: Send + 'static + Clone, F> crate::traits::error::Error
  for ReduceTransformer<T, Acc, F>
where
  F: FnMut(Acc, T) -> Acc + Send + 'static + Clone,
{
  type Error = StreamError<T>;
}

impl<T: Send + 'static + Clone, Acc: Send + 'static + Clone, F> Input
  for ReduceTransformer<T, Acc, F>
where
  F: FnMut(Acc, T) -> Acc + Send + 'static + Clone,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError<T>>> + Send>>;
}

impl<T: Send + 'static + Clone, Acc: Send + 'static + Clone, F> Output
  for ReduceTransformer<T, Acc, F>
where
  F: FnMut(Acc, T) -> Acc + Send + 'static + Clone,
{
  type Output = Acc;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError<T>>> + Send>>;
}

#[async_trait]
impl<T: Send + 'static + Clone, Acc: Send + 'static + Clone, F> Transformer
  for ReduceTransformer<T, Acc, F>
where
  F: FnMut(Acc, T) -> Acc + Send + 'static + Clone,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let mut accumulator = self.accumulator.clone();
    let mut reducer = self.reducer.clone();
    Box::pin(input.map(move |result| {
      result.map(|item| {
        accumulator = reducer(accumulator, item);
        accumulator.clone()
      })
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
        .unwrap_or_else(|| "reduce_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::TryStreamExt;
  use futures::stream;

  #[tokio::test]
  async fn test_reduce_transformer_sum() {
    let mut transformer = ReduceTransformer::new(0, |acc, x| acc + x);
    let input = vec![1, 2, 3, 4, 5];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<i32> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 3, 6, 10, 15]);
  }

  #[tokio::test]
  async fn test_reduce_transformer_string_concat() {
    let mut transformer = ReduceTransformer::new(String::new(), |acc, x| acc + x);
    let input = vec!["a", "b", "c"];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<String> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(
      result,
      vec!["a".to_string(), "ab".to_string(), "abc".to_string()]
    );
  }

  #[tokio::test]
  async fn test_reduce_transformer_with_error() {
    let mut transformer =
      ReduceTransformer::new(0, |acc, x| if x % 2 == 0 { acc + x } else { acc });

    let input = vec![2, 3, 4, 5, 6];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result = transformer
      .transform(input_stream)
      .try_collect::<Vec<_>>()
      .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vec![2, 2, 6, 6, 12]);
  }

  #[tokio::test]
  async fn test_reduce_transformer_custom_type() {
    #[derive(Debug, Clone, PartialEq)]
    struct Counter {
      count: i32,
      sum: i32,
    }

    let mut transformer = ReduceTransformer::new(Counter { count: 0, sum: 0 }, |acc, x| Counter {
      count: acc.count + 1,
      sum: acc.sum + x,
    });

    let input = vec![1, 2, 3, 4, 5];
    let input_stream = Box::pin(stream::iter(input.into_iter().map(Ok)));

    let result: Vec<Counter> = transformer
      .transform(input_stream)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(
      result,
      vec![
        Counter { count: 1, sum: 1 },
        Counter { count: 2, sum: 3 },
        Counter { count: 3, sum: 6 },
        Counter { count: 4, sum: 10 },
        Counter { count: 5, sum: 15 },
      ]
    );
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = ReduceTransformer::new(0, |acc, x| acc + x)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
