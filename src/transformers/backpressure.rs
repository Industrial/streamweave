use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

pub struct BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  buffer_size: usize,
  config: TransformerConfig<T>,
}

impl<T> BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(buffer_size: usize) -> Self {
    Self {
      buffer_size,
      config: TransformerConfig::default(),
    }
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
    self.buffer_size = buffer_size;
    self
  }
}

impl<T> Clone for BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      buffer_size: self.buffer_size,
      config: self.config.clone(),
    }
  }
}

impl<T> Input for BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for BackpressureTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(
      input
        .ready_chunks(self.buffer_size)
        .flat_map(|chunks| futures::stream::iter(chunks)),
    )
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "backpressure_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use tokio_stream::wrappers::ReceiverStream;

  #[tokio::test]
  async fn test_backpressure_transformer_basic() {
    let mut transformer = BackpressureTransformer::new(3);
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send items
    for i in 0..5 {
      tx.send(i).await.unwrap();
    }
    drop(tx);

    // Collect output
    let mut result = Vec::new();
    while let Some(item) = output.next().await {
      result.push(item);
    }

    // Should have all 5 items
    assert_eq!(result, vec![0, 1, 2, 3, 4]);
  }

  #[tokio::test]
  async fn test_backpressure_transformer_small_buffer() {
    let mut transformer = BackpressureTransformer::new(2);
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send items
    for i in 0..4 {
      tx.send(i).await.unwrap();
    }
    drop(tx);

    // Collect output
    let mut result = Vec::new();
    while let Some(item) = output.next().await {
      result.push(item);
    }

    // Should have all 4 items
    assert_eq!(result, vec![0, 1, 2, 3]);
  }

  #[tokio::test]
  async fn test_backpressure_transformer_empty_input() {
    let mut transformer: BackpressureTransformer<i32> = BackpressureTransformer::new(5);
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send no items
    drop(tx);

    // Collect output
    let mut result = Vec::new();
    while let Some(item) = output.next().await {
      result.push(item);
    }

    // Should be empty
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_backpressure_transformer_strings() {
    let mut transformer: BackpressureTransformer<String> = BackpressureTransformer::new(3);
    let (tx, rx) = tokio::sync::mpsc::channel(10);

    let input = Box::pin(ReceiverStream::new(rx));
    let mut output = transformer.transform(input);

    // Send string items
    let strings = vec!["hello", "world", "test", "backpressure"];
    for s in strings.iter() {
      tx.send(s.to_string()).await.unwrap();
    }
    drop(tx);

    // Collect output
    let mut result = Vec::new();
    while let Some(item) = output.next().await {
      result.push(item);
    }

    // Should have all strings
    assert_eq!(result, strings);
  }

  #[test]
  fn test_backpressure_transformer_config() {
    let transformer: BackpressureTransformer<i32> =
      BackpressureTransformer::new(10).with_name("test_backpressure".to_string());

    assert_eq!(transformer.buffer_size, 10);
    assert_eq!(
      transformer.config.name(),
      Some("test_backpressure".to_string())
    );
  }
}
