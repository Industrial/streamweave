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
use rand::Rng;
use std::pin::Pin;

pub struct SampleTransformer<T>
where
  T: Send + 'static + Clone,
{
  probability: f64,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> SampleTransformer<T>
where
  T: Send + 'static + Clone,
{
  pub fn new(probability: f64) -> Self {
    assert!(
      probability >= 0.0 && probability <= 1.0,
      "Probability must be between 0 and 1"
    );
    Self {
      probability,
      config: TransformerConfig::default(),
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

impl<T> Input for SampleTransformer<T>
where
  T: Send + 'static + Clone,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for SampleTransformer<T>
where
  T: Send + 'static + Clone,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for SampleTransformer<T>
where
  T: Send + 'static + Clone,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let probability = self.probability;
    Box::pin(input.filter(move |_| {
      let mut rng = rand::thread_rng();
      futures::future::ready(rng.gen_bool(probability))
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
        .unwrap_or_else(|| "sample_transformer".to_string()),
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
  async fn test_sample_basic() {
    let mut transformer = SampleTransformer::new(0.5);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    // Since we're sampling with 0.5 probability, the result should be a subset
    assert!(result.len() <= 5);
    assert!(result.iter().all(|&x| x >= 1 && x <= 5));
  }

  #[tokio::test]
  async fn test_sample_empty_input() {
    let mut transformer = SampleTransformer::new(0.5);
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_sample_with_error() {
    let mut transformer = SampleTransformer::new(0.5);
    let input = stream::iter(vec![1, 2, 3].into_iter().map(|x| {
      if x == 2 {
        Err(StreamError::new(
          Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
          ErrorContext {
            timestamp: chrono::Utc::now(),
            item: None,
            stage: PipelineStage::Transformer("test".to_string()),
          },
          ComponentInfo {
            name: "test".to_string(),
            type_name: "test".to_string(),
          },
        ))
      } else {
        Ok(x)
      }
    }));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;
    assert!(result.len() <= 3);
    assert!(result.iter().all(|&x| x == 1 || x == 3));
  }

  #[tokio::test]
  async fn test_sample_probability_0() {
    let mut transformer = SampleTransformer::new(0.0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_sample_probability_1() {
    let mut transformer = SampleTransformer::new(1.0);
    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = SampleTransformer::new(0.5)
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
