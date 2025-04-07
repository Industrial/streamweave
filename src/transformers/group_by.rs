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
use std::collections::HashMap;
use std::hash::Hash;
use std::pin::Pin;

pub struct GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + 'static,
  T: Send + 'static + Clone,
  K: Eq + Hash + Send + 'static,
{
  key_fn: F,
  config: TransformerConfig<T>,
  _phantom_t: std::marker::PhantomData<T>,
  _phantom_k: std::marker::PhantomData<K>,
}

impl<F, T, K> GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + 'static,
  T: Send + 'static + Clone,
  K: Eq + Hash + Send + 'static,
{
  pub fn new(key_fn: F) -> Self {
    Self {
      key_fn,
      config: TransformerConfig::default(),
      _phantom_t: std::marker::PhantomData,
      _phantom_k: std::marker::PhantomData,
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

impl<F, T, K> Input for GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + 'static,
  T: Send + 'static + Clone,
  K: Eq + Hash + Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<F, T, K> Output for GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + 'static,
  T: Send + 'static + Clone,
  K: Eq + Hash + Send + 'static,
{
  type Output = (K, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (K, Vec<T>)> + Send>>;
}

#[async_trait]
impl<F, T, K> Transformer for GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + 'static,
  T: Send + 'static + Clone,
  K: Eq + Hash + Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let key_fn = &self.key_fn;
    Box::pin(
      input
        .fold(HashMap::new(), move |mut groups, item| async move {
          let key = key_fn(&item);
          groups.entry(key).or_insert_with(Vec::new).push(item);
          groups
        })
        .then(|groups| async move { futures::stream::iter(groups.into_iter()) }),
    )
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
        .unwrap_or_else(|| "group_by_transformer".to_string()),
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
  async fn test_group_by_basic() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let mut result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    // Sort the results for consistent comparison
    result.sort_by_key(|&(k, _)| k);

    assert_eq!(result, vec![(0, vec![2, 4]), (1, vec![1, 3, 5]),]);
  }

  #[tokio::test]
  async fn test_group_by_empty_input() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(i32, Vec<i32>)> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<(i32, Vec<i32>)>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 2)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
