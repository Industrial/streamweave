use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::filter::filter_transformer::FilterTransformer;
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<F, T> Transformer for FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let predicate = self.predicate.clone();
    Box::pin(input.filter(move |item| {
      let mut predicate = predicate.clone();
      futures::future::ready(predicate(item))
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
        .unwrap_or_else(|| "filter_transformer".to_string()),
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
  async fn test_filter_even_numbers() {
    let mut transformer = FilterTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![2, 4, 6]);
  }

  #[tokio::test]
  async fn test_filter_empty_input() {
    let mut transformer = FilterTransformer::new(|x: &i32| x % 2 == 0);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_filter_all_match() {
    let mut transformer = FilterTransformer::new(|x: &i32| *x > 0);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_filter_none_match() {
    let mut transformer = FilterTransformer::new(|x: &i32| *x > 10);
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

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
      .into_iter(),
    );
    let boxed_input = Box::pin(input);

    let result: Vec<String> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec!["apple".to_string(), "avocado".to_string()]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = FilterTransformer::new(|_: &i32| true)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
