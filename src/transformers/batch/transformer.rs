use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::batch::batch_transformer::BatchTransformer;
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T> Transformer for BatchTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, mut input: Self::InputStream) -> Self::OutputStream {
    let size = self.size;
    let mut current_batch: Vec<T> = Vec::with_capacity(size);

    Box::pin(async_stream::stream! {
      while let Some(item) = input.next().await {
        current_batch.push(item);
        if current_batch.len() == size {
          yield current_batch;
          current_batch = Vec::with_capacity(size);
        }
      }
      if !current_batch.is_empty() {
        yield current_batch;
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
        .unwrap_or_else(|| "batch_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_batch_exact_size() {
    let mut transformer = BatchTransformer::new(3).unwrap();
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, 2, 3], vec![4, 5, 6]]);
  }

  #[tokio::test]
  async fn test_batch_partial_last_chunk() {
    let mut transformer = BatchTransformer::new(2).unwrap();
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, 2], vec![3, 4], vec![5]]);
  }

  #[tokio::test]
  async fn test_batch_empty_input() {
    let mut transformer = BatchTransformer::new(2).unwrap();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_batch_size_one() {
    let mut transformer = BatchTransformer::new(1).unwrap();
    let input = stream::iter(vec![1, 2, 3].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1], vec![2], vec![3]]);
  }

  #[tokio::test]
  async fn test_batch_size_larger_than_input() {
    let mut transformer = BatchTransformer::new(10).unwrap();
    let input = stream::iter(vec![1, 2, 3].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![vec![1, 2, 3]]);
  }

  #[test]
  fn test_batch_invalid_size() {
    let result = BatchTransformer::<i32>::new(0);
    assert!(result.is_err());
  }

  #[test]
  fn test_error_handling_strategies() {
    let transformer: BatchTransformer<i32> = BatchTransformer::new(2).unwrap();
    assert_eq!(transformer.config.error_strategy, ErrorStrategy::Stop);
  }
}
