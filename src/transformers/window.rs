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
use tokio::time::{Duration, Instant};

pub struct WindowTransformer<T: Send + 'static + Clone> {
  size: usize,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + 'static + Clone> WindowTransformer<T> {
  pub fn new(size: usize) -> Self {
    Self {
      size,
      config: TransformerConfig::<T>::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T: Send + 'static + Clone> crate::traits::error::Error for WindowTransformer<T> {
  type Error = StreamError<T>;
}

impl<T: Send + 'static + Clone> Input for WindowTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError<T>>> + Send>>;
}

impl<T: Send + 'static + Clone> Output for WindowTransformer<T> {
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError<T>>> + Send>>;
}

#[async_trait]
impl<T: Send + 'static + Clone> Transformer for WindowTransformer<T> {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let size = self.size;
    Box::pin(input.chunks(size).map(|chunk| {
      let mut window = Vec::with_capacity(size);
      for item in chunk {
        match item {
          Ok(item) => window.push(item),
          Err(e) => return Err(e),
        }
      }
      Ok(window)
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
    match self.config.error_strategy() {
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
      stage: PipelineStage::Transformer(self.component_info().name),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "window_transformer".to_string()),
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
  async fn test_window_basic() {
    let mut transformer = WindowTransformer::new(3);
    let input = stream::iter(vec![1, 2, 3, 4, 5, 6, 7].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![vec![1, 2, 3], vec![4, 5, 6], vec![7],]);
  }

  #[tokio::test]
  async fn test_window_empty_input() {
    let mut transformer = WindowTransformer::new(3);
    let input = stream::iter(Vec::<Result<i32, StreamError<i32>>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<Vec<i32>> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_window_with_error() {
    let mut transformer = WindowTransformer::new(3);
    let input = stream::iter(vec![
      Ok(1),
      Err(StreamError {
        source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Transformer("test".to_string()),
        },
        retries: 0,
        component: ComponentInfo {
          name: "test".to_string(),
          type_name: "test".to_string(),
        },
      }),
      Ok(3),
    ]);
    let boxed_input = Box::pin(input);

    let result: Result<Vec<Vec<i32>>, _> = transformer.transform(boxed_input).try_collect().await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = WindowTransformer::new(3)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
