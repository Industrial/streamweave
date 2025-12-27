use crate::split_transformer::SplitTransformer;
use async_trait::async_trait;
use futures::stream;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use streamweave_core::{Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[async_trait]
impl<F, T> Transformer for SplitTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (Vec<T>,);

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let predicate = self.predicate.clone();

    Box::pin(futures::stream::unfold(
      (input, predicate),
      |(mut input, mut pred)| async move {
        if let Some(items) = input.next().await {
          let mut current_chunk = Vec::new();
          let mut chunks = Vec::new();

          for item in items {
            if pred(&item) && !current_chunk.is_empty() {
              chunks.push(std::mem::take(&mut current_chunk));
            }
            current_chunk.push(item);
          }

          if !current_chunk.is_empty() {
            chunks.push(current_chunk);
          }

          if chunks.is_empty() {
            Some((Vec::new(), (input, pred)))
          } else {
            let mut iter = chunks.into_iter();
            let first = iter.next().unwrap();
            Some((
              first,
              (
                Box::pin(stream::iter(iter).chain(input))
                  as Pin<Box<dyn Stream<Item = Vec<T>> + Send>>,
                pred,
              ),
            ))
          }
        } else {
          None
        }
      },
    ))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Vec<T>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Vec<T>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Vec<T>> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Vec<T>>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Vec<T>>) -> ErrorContext<Vec<T>> {
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
        .unwrap_or_else(|| "split_transformer".to_string()),
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
  async fn test_split_by_even_numbers() {
    let mut transformer = SplitTransformer::new(|x: &i32| x % 2 == 0);
    let input = vec![vec![1, 2, 3, 4, 5, 6]];
    let input_stream = Box::pin(stream::iter(input.into_iter()));

    let result: Vec<Vec<i32>> = transformer.transform(input_stream).collect().await;

    assert_eq!(result, vec![vec![1], vec![2, 3], vec![4, 5], vec![6]]);
  }

  #[tokio::test]
  async fn test_split_no_splits() {
    let mut transformer = SplitTransformer::new(|x: &i32| *x < 0); // No negatives in input
    let input = vec![vec![1, 2, 3, 4, 5]];
    let input_stream = Box::pin(stream::iter(input.into_iter()));

    let result: Vec<Vec<i32>> = transformer.transform(input_stream).collect().await;

    assert_eq!(result, vec![vec![1, 2, 3, 4, 5]]);
  }

  #[tokio::test]
  async fn test_split_strings() {
    let mut transformer = SplitTransformer::new(|s: &String| s.is_empty());
    let input = vec![vec![
      "hello".to_string(),
      "".to_string(),
      "world".to_string(),
      "".to_string(),
      "rust".to_string(),
    ]];
    let input_stream = Box::pin(stream::iter(input.into_iter()));

    let result: Vec<Vec<String>> = transformer.transform(input_stream).collect().await;

    assert_eq!(
      result,
      vec![
        vec!["hello".to_string()],
        vec!["".to_string(), "world".to_string()],
        vec!["".to_string(), "rust".to_string()],
      ]
    );
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let transformer = SplitTransformer::new(|_: &i32| true)
      .with_error_strategy(ErrorStrategy::<Vec<i32>>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<Vec<i32>>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
