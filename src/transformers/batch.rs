use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::TryStreamExt;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct BatchTransformer<T> {
  size: usize,
  config: TransformerConfig,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> BatchTransformer<T>
where
  T: Send + 'static,
{
  pub fn new(size: usize) -> Result<Self, StreamError> {
    if size == 0 {
      return Err(StreamError::new(
        Box::new(std::io::Error::new(
          std::io::ErrorKind::InvalidInput,
          "Batch size must be greater than zero",
        )),
        ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Transformer,
        },
        ComponentInfo {
          name: "batch_transformer".to_string(),
          type_name: std::any::type_name::<Self>().to_string(),
        },
      ));
    }
    Ok(Self {
      size,
      config: TransformerConfig::default(),
      _phantom: std::marker::PhantomData,
    })
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

impl<T: Send + 'static> crate::traits::error::Error for BatchTransformer<T> {
  type Error = StreamError;
}

impl<T: Send + 'static> Input for BatchTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<T: Send + 'static> Output for BatchTransformer<T> {
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<T: Send + 'static> Transformer for BatchTransformer<T> {
  fn transform(&mut self, mut input: Self::InputStream) -> Self::OutputStream {
    let size = self.size;
    let mut current_batch: Vec<T> = Vec::with_capacity(size);

    let stream = async_stream::try_stream! {
        while let Some(result) = input.next().await {
            let item = result?;
            current_batch.push(item);

            if current_batch.len() >= size {
                yield current_batch.split_off(0);
            }
        }

        // Emit any remaining items
        if !current_batch.is_empty() {
            yield current_batch;
        }
    };

    Box::pin(stream)
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
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1, 2, 3], vec![4, 5, 6]]);
  }

  #[tokio::test]
  async fn test_batch_partial_last_chunk() {
    let mut transformer = BatchTransformer::new(2).unwrap();
    let input = stream::iter(vec![1, 2, 3, 4, 5].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1, 2], vec![3, 4], vec![5]]);
  }

  #[tokio::test]
  async fn test_batch_empty_input() {
    let mut transformer = BatchTransformer::new(2).unwrap();
    let input = stream::iter(Vec::<Result<i32, StreamError>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_batch_size_one() {
    let mut transformer = BatchTransformer::new(1).unwrap();
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1], vec![2], vec![3]]);
  }

  #[tokio::test]
  async fn test_batch_size_larger_than_input() {
    let mut transformer = BatchTransformer::new(5).unwrap();
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1, 2, 3]]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = BatchTransformer::new(2)
      .unwrap()
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
