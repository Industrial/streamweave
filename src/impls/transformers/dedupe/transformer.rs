use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::transformers::dedupe::DedupeTransformer;
use crate::traits::transformer::{Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::StreamExt;
use std::hash::Hash;

#[async_trait]
impl<T> Transformer for DedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(
      futures::stream::unfold((input, self.seen.clone()), |state| async move {
        let (mut input, mut seen) = state;

        match input.next().await {
          Some(item) => {
            let is_new = seen.insert(item.clone());
            if is_new {
              Some((Some(item), (input, seen)))
            } else {
              // Skip duplicates by continuing to next item
              Some((None, (input, seen)))
            }
          }
          None => None,
        }
      })
      .filter_map(|x| futures::future::ready(x)),
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
        .unwrap_or_else(|| "dedupe_transformer".to_string()),
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
  async fn test_dedupe_transformer() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(vec![1, 2, 2, 3, 3, 3, 4].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4]);
  }

  #[tokio::test]
  async fn test_dedupe_transformer_strings() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(vec!["a", "b", "b", "c", "a"].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<&str> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec!["a", "b", "c"]);
  }

  #[tokio::test]
  async fn test_dedupe_transformer_empty() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_dedupe_transformer_all_duplicates() {
    let mut transformer = DedupeTransformer::new();
    let input = stream::iter(vec![1, 1, 1, 1].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = DedupeTransformer::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
