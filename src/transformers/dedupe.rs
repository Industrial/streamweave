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
use std::collections::HashSet;
use std::pin::Pin;

pub struct DedupeTransformer<T> {
  seen: HashSet<T>,
  config: TransformerConfig,
}

impl<T> DedupeTransformer<T>
where
  T: Eq + std::hash::Hash + Clone + Send + 'static,
{
  pub fn new() -> Self {
    Self {
      seen: HashSet::new(),
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

impl<T: Send + 'static> crate::traits::error::Error for DedupeTransformer<T> {
  type Error = StreamError;
}

impl<T: Send + 'static> Input for DedupeTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<T: Send + 'static> Output for DedupeTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<T> Transformer for DedupeTransformer<T>
where
  T: Eq + std::hash::Hash + Clone + Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(
      futures::stream::unfold((input, self.seen.clone()), |mut state| async move {
        let (mut input, mut seen) = state;

        match input.next().await {
          Some(Ok(item)) => {
            let is_new = seen.insert(item.clone());
            let result = if is_new {
              Ok(item)
            } else {
              // Skip duplicates by continuing to next item
              return Some((None, (input, seen)));
            };
            Some((Some(result), (input, seen)))
          }
          Some(Err(e)) => Some((Some(Err(e)), (input, seen))),
          None => None,
        }
      })
      .filter_map(|x| futures::future::ready(x)),
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
        .unwrap_or_else(|| "dedupe_transformer".to_string()),
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
  async fn test_dedupe_transformer() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(vec![1, 2, 2, 3, 3, 3, 4].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 2, 3, 4]);
  }

  #[tokio::test]
  async fn test_dedupe_transformer_strings() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(vec!["a", "b", "b", "c", "a"].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<&str> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec!["a", "b", "c"]);
  }

  #[tokio::test]
  async fn test_dedupe_transformer_empty() {
    let mut transformer = DedupeTransformer::new();
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
  async fn test_dedupe_transformer_all_duplicates() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(vec![1, 1, 1, 1].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = DedupeTransformer::new()
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
