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

pub struct GroupByTransformer<F, T, K> {
  key_fn: F,
  config: TransformerConfig,
  _phantom_t: std::marker::PhantomData<T>,
  _phantom_k: std::marker::PhantomData<K>,
}

impl<F, T, K> GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + 'static,
  T: Send + 'static,
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

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    self.config_mut().set_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config_mut().set_name(name);
    self
  }
}

impl<F, T, K> crate::traits::error::Error for GroupByTransformer<F, T, K>
where
  F: Send + 'static,
  T: Send + 'static,
  K: Send + 'static,
{
  type Error = StreamError;
}

impl<F, T, K> Input for GroupByTransformer<F, T, K>
where
  F: Send + 'static,
  T: Send + 'static,
  K: Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<F, T, K> Output for GroupByTransformer<F, T, K>
where
  F: Send + 'static,
  T: Send + 'static,
  K: Send + 'static,
{
  type Output = (K, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<F, T, K> Transformer for GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + 'static,
  T: Send + 'static,
  K: Eq + Hash + Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let key_fn = &self.key_fn;
    Box::pin(
      input
        .fold(HashMap::new(), move |mut groups, result| async move {
          match result {
            Ok(item) => {
              let key = key_fn(&item);
              groups.entry(key).or_insert_with(Vec::new).push(item);
              groups
            }
            Err(e) => {
              // If we encounter an error, we'll return it in the next stream
              groups.insert(key_fn(&item), vec![item]);
              groups
            }
          }
        })
        .map(|groups| Ok(groups.into_iter().collect::<Vec<_>>())),
    )
  }

  fn config(&self) -> &TransformerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut TransformerConfig {
    &mut self.config
  }

  fn handle_error(&self, error: StreamError) -> ErrorStrategy {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorStrategy::Retry(n),
      _ => ErrorStrategy::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Transformer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "group_by_transformer".to_string()),
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
  async fn test_group_by_basic() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let mut result: Vec<(i32, Vec<i32>)> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    // Sort the results for consistent comparison
    result.sort_by_key(|&(k, _)| k);

    assert_eq!(result, vec![(0, vec![2, 4]), (1, vec![1, 3, 5]),]);
  }

  #[tokio::test]
  async fn test_group_by_empty_input() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
    let input = stream::iter(Vec::<Result<i32, StreamError>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(i32, Vec<i32>)> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<(i32, Vec<i32>)>::new());
  }

  #[tokio::test]
  async fn test_group_by_with_error() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 2);
    let input = stream::iter(vec![
      Ok(1),
      Err(StreamError::new(
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
        ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Transformer,
        },
        ComponentInfo {
          name: "test".to_string(),
          type_name: "test".to_string(),
        },
      )),
      Ok(3),
    ]);
    let boxed_input = Box::pin(input);

    let result: Result<Vec<(i32, Vec<i32>)>, _> =
      transformer.transform(boxed_input).try_collect().await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = GroupByTransformer::new(|x: &i32| x % 2)
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      transformer.create_error_context(None),
      transformer.component_info(),
    );

    assert_eq!(transformer.handle_error(error), ErrorStrategy::Skip);
  }
}
