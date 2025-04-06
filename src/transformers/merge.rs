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

pub struct MergeTransformer<T> {
  _phantom: std::marker::PhantomData<T>,
  config: TransformerConfig,
}

impl<T> MergeTransformer<T>
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

impl<T> Default for MergeTransformer<T>
where
  T: Send + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T: Send + 'static> crate::traits::error::Error for MergeTransformer<T> {
  type Error = StreamError;
}

impl<T: Send + 'static> Input for MergeTransformer<T> {
  type Input = Vec<Pin<Box<dyn Stream<Item = Result<T, StreamError>> + Send>>>;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<T: Send + 'static> Output for MergeTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<T> Transformer for MergeTransformer<T>
where
  T: Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(futures::stream::unfold(input, |mut input| async move {
      while let Some(streams_result) = input.next().await {
        match streams_result {
          Ok(streams) => {
            if streams.is_empty() {
              continue;
            }

            let mut all_items = Vec::new();
            for mut stream in streams {
              while let Some(item_result) = stream.next().await {
                match item_result {
                  Ok(item) => all_items.push(item),
                  Err(e) => return Some((Err(e), input)),
                }
              }
            }

            // Process items one at a time
            if !all_items.is_empty() {
              let item = all_items.remove(0);
              // If there are remaining items, create a new stream with them
              if !all_items.is_empty() {
                let remaining_stream = Box::pin(futures::stream::iter(
                  all_items.into_iter().map(|item| Ok::<T, StreamError>(item)),
                ));
                let mut new_streams = Vec::new();
                new_streams.push(remaining_stream);
                input.next().await; // Consume the current streams
              }
              return Some((Ok(item), input));
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
        .unwrap_or_else(|| "merge_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::TryStreamExt;
  use futures::stream;

  fn create_stream<T: Send + 'static>(
    items: Vec<T>,
  ) -> Pin<Box<dyn Stream<Item = Result<T, StreamError>> + Send>> {
    Box::pin(stream::iter(items.into_iter().map(Ok)))
  }

  #[tokio::test]
  async fn test_merge_empty_input() {
    let mut transformer = MergeTransformer::<i32>::new();
    let input = stream::iter(vec![Ok(Vec::new())]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_merge_empty_streams() {
    let mut transformer = MergeTransformer::<i32>::new();
    let streams = vec![
      create_stream(Vec::<i32>::new()),
      create_stream(Vec::<i32>::new()),
      create_stream(Vec::<i32>::new()),
    ];
    let input = stream::iter(vec![Ok(streams)]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = MergeTransformer::<i32>::new()
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
