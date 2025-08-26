use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_stream::stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  accumulator: Acc,
  reducer: F,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T, Acc, F> ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  pub fn new(initial: Acc, reducer: F) -> Self {
    Self {
      accumulator: initial,
      reducer,
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

impl<T, Acc, F> Input for ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T, Acc, F> Output for ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  type Output = Acc;
  type OutputStream = Pin<Box<dyn Stream<Item = Acc> + Send>>;
}

#[async_trait]
impl<T, Acc, F> Transformer for ReduceTransformer<T, Acc, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Acc: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: FnMut(Acc, T) -> Acc + Send + Clone + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let mut reducer = self.reducer.clone();
    let initial = self.accumulator.clone();
    Box::pin(async_stream::stream! {
      let mut acc = initial;
      let mut input = input;

      while let Some(item) = input.next().await {
        acc = reducer(acc.clone(), item);
        yield acc.clone();
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
        .unwrap_or_else(|| "reduce_transformer".to_string()),
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
  async fn test_reduce_transformer_sum() {
    let mut transformer = ReduceTransformer::new(0, |acc, x| acc + x);
    let input = vec![1, 2, 3, 4, 5];
    let input_stream = Box::pin(stream::iter(input.into_iter()));

    let result: Vec<i32> = transformer.transform(input_stream).collect().await;

    assert_eq!(result, vec![1, 3, 6, 10, 15]);
  }

  #[tokio::test]
  async fn test_reduce_transformer_string_concat() {
    let mut transformer = ReduceTransformer::new(String::new(), |acc, x| acc + x);
    let input = vec!["a", "b", "c"];
    let input_stream = Box::pin(stream::iter(input.into_iter()));

    let result: Vec<String> = transformer.transform(input_stream).collect().await;

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
    let input_stream = Box::pin(stream::iter(input.into_iter()));

    let result = transformer
      .transform(input_stream)
      .collect::<Vec<_>>()
      .await;

    assert_eq!(result, vec![2, 2, 6, 6, 12]);
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
    let input_stream = Box::pin(stream::iter(input.into_iter()));

    let result: Vec<Counter> = transformer.transform(input_stream).collect().await;

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
