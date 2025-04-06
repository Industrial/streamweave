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

pub struct FilterTransformer<F, T> {
  predicate: F,
  _phantom: std::marker::PhantomData<T>,
  config: TransformerConfig,
}

impl<F, T> FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: Send + 'static,
{
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      _phantom: std::marker::PhantomData,
      config: TransformerConfig::default(),
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

impl<F, T: Send + 'static> crate::traits::error::Error for FilterTransformer<F, T> {
  type Error = StreamError;
}

impl<F, T: Send + 'static> Input for FilterTransformer<F, T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<F, T: Send + 'static> Output for FilterTransformer<F, T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<F, T> Transformer for FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: Send + Clone + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let mut predicate = self.predicate.clone();

    Box::pin(futures::stream::unfold(input, move |mut input| {
      let mut predicate = predicate.clone();
      async move {
        while let Some(result) = input.next().await {
          match result {
            Ok(item) => {
              if predicate(&item) {
                return Some((Ok(item), input));
              }
              // Continue to next item if predicate returns false
              continue;
            }
            Err(e) => return Some((Err(e), input)),
          }
        }
        None
      }
    }))
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
        .unwrap_or_else(|| "filter_transformer".to_string()),
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
  async fn test_filter_even_numbers() {
    let mut transformer = FilterTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![2, 4, 6]);
  }

  #[tokio::test]
  async fn test_filter_empty_input() {
    let mut transformer = FilterTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(Vec::<Result<i32, StreamError>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_filter_all_match() {
    let mut transformer = FilterTransformer::new(|x: &i32| *x > 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_filter_none_match() {
    let mut transformer = FilterTransformer::new(|x: &i32| *x > 10);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_filter_with_strings() {
    let mut transformer = FilterTransformer::new(|s: &String| s.starts_with("a"));
    let input = stream::iter(
      vec![
        "apple".to_string(),
        "banana".to_string(),
        "avocado".to_string(),
        "cherry".to_string(),
      ]
      .into_iter()
      .map(Ok),
    );
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec!["apple".to_string(), "avocado".to_string()]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = FilterTransformer::new(|_: &i32| true)
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
