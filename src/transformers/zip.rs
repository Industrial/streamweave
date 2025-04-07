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

pub struct ZipTransformer<T: Send + 'static + Clone, U: Send + 'static + Clone> {
  other: Pin<Box<dyn Stream<Item = Result<U, StreamError<U>>> + Send>>,
  config: TransformerConfig<T>,
  _phantom_t: std::marker::PhantomData<T>,
  _phantom_u: std::marker::PhantomData<U>,
}

impl<T: Send + 'static + Clone, U: Send + 'static + Clone> ZipTransformer<T, U> {
  pub fn new(other: Pin<Box<dyn Stream<Item = Result<U, StreamError<U>>> + Send>>) -> Self {
    Self {
      other,
      config: TransformerConfig::<T>::default(),
      _phantom_t: std::marker::PhantomData,
      _phantom_u: std::marker::PhantomData,
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

impl<T: Send + 'static + Clone, U: Send + 'static + Clone> crate::traits::error::Error
  for ZipTransformer<T, U>
{
  type Error = StreamError<T>;
}

impl<T: Send + 'static + Clone, U: Send + 'static + Clone> Input for ZipTransformer<T, U> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError<T>>> + Send>>;
}

impl<T: Send + 'static + Clone, U: Send + 'static + Clone> Output for ZipTransformer<T, U> {
  type Output = (T, U);
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError<T>>> + Send>>;
}

#[async_trait]
impl<T: Send + 'static + Clone, U: Send + 'static + Clone> Transformer for ZipTransformer<T, U> {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let other = self.other.clone();
    Box::pin(input.zip(other).map(|(a, b)| match (a, b) {
      (Ok(a), Ok(b)) => Ok((a, b)),
      (Err(e), _) => Err(e),
      (_, Err(e)) => Err(StreamError {
        source: e.source,
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Transformer(self.component_info().name),
        },
        retries: 0,
        component: self.component_info(),
      }),
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
        .unwrap_or_else(|| "zip_transformer".to_string()),
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
  async fn test_zip_basic() {
    let other = stream::iter(vec!['a', 'b', 'c'].into_iter().map(Ok));
    let mut transformer = ZipTransformer::new(Box::pin(other));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<(i32, char)> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![(1, 'a'), (2, 'b'), (3, 'c')]);
  }

  #[tokio::test]
  async fn test_zip_empty_input() {
    let other = stream::iter(Vec::<Result<char, StreamError<char>>>::new());
    let mut transformer = ZipTransformer::new(Box::pin(other));
    let input = stream::iter(Vec::<Result<i32, StreamError<i32>>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(i32, char)> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<(i32, char)>::new());
  }

  #[tokio::test]
  async fn test_zip_with_error() {
    let other = stream::iter(vec![Ok('a'), Ok('b')]);
    let mut transformer = ZipTransformer::new(Box::pin(other));
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
      Ok(2),
    ]);
    let boxed_input = Box::pin(input);

    let result: Result<Vec<(i32, char)>, _> =
      transformer.transform(boxed_input).try_collect().await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let other = stream::iter(vec![Ok('a')]);
    let mut transformer = ZipTransformer::new(Box::pin(other))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
