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

pub struct FlattenTransformer<T> {
  _phantom: std::marker::PhantomData<T>,
  config: TransformerConfig,
}

impl<T> FlattenTransformer<T>
where
  T: Send + 'static,
{
  pub fn new() -> Self {
    Self {
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

impl<T: Send + 'static> crate::traits::error::Error for FlattenTransformer<T> {
  type Error = StreamError;
}

impl<T: Send + 'static> Input for FlattenTransformer<T> {
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<T: Send + 'static> Output for FlattenTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<T> Transformer for FlattenTransformer<T>
where
  T: Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(futures::stream::unfold(input, |mut input| async move {
      while let Some(result) = input.next().await {
        match result {
          Ok(vec) => {
            if vec.is_empty() {
              continue;
            }
            // Create an iterator over the vector's items
            let mut items = vec.into_iter();
            // Get the first item
            if let Some(first) = items.next() {
              // Return the first item and store remaining items for next iteration
              let remaining = items.collect::<Vec<_>>();
              if !remaining.is_empty() {
                // Push remaining items back to input stream
                return Some((Ok(first), input));
              }
              return Some((Ok(first), input));
            }
          }
          Err(e) => return Some((Err(e), input)),
        }
      }
      None
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
        .unwrap_or_else(|| "flatten_transformer".to_string()),
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
  async fn test_empty_input_vectors() {
    let mut transformer = FlattenTransformer::new();
    let input = stream::iter(vec![Ok(vec![]), Ok(vec![]), Ok(vec![1]), Ok(vec![])]);
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
    let mut transformer = FlattenTransformer::new()
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
