use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  error::Error,
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;

pub struct VecProducer<T> {
  data: Vec<T>,
  config: ProducerConfig,
}

impl<T: Clone + Send + 'static> VecProducer<T> {
  pub fn new(data: Vec<T>) -> Self {
    Self {
      data,
      config: ProducerConfig::default(),
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

impl<T: Clone + Send + 'static> Error for VecProducer<T> {
  type Error = StreamError;
}

impl<T: Clone + Send + 'static> Output for VecProducer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<T, StreamError>> + Send>>;
}

impl<T: Clone + Send + 'static> Producer for VecProducer<T> {
  fn produce(&mut self) -> Self::OutputStream {
    let stream = futures::stream::iter(self.data.clone().into_iter().map(Ok));
    Box::pin(stream)
  }

  fn config(&self) -> &ProducerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut ProducerConfig {
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

  fn create_error_context(
    &self,
    item: Option<Arc<dyn std::any::Any + Send + Sync>>,
  ) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Producer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "vec_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_vec_producer() {
    let data = vec!["test1".to_string(), "test2".to_string()];
    let mut producer = VecProducer::new(data.clone());
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, data);
  }

  #[tokio::test]
  async fn test_vec_producer_empty() {
    let mut producer = VecProducer::<String>::new(vec![]);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, Vec::<String>::new());
  }

  #[tokio::test]
  async fn test_vec_producer_multiple_calls() {
    let data = vec!["test1".to_string(), "test2".to_string()];
    let mut producer = VecProducer::new(data.clone());

    // First call
    let stream = producer.produce();
    let result1: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1, data);

    // Second call
    let stream = producer.produce();
    let result2: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2, data);
  }

  #[tokio::test]
  async fn test_vec_producer_clone_independence() {
    let mut producer1 = VecProducer::new(vec![1, 2, 3]);
    let mut producer2 = VecProducer::new(vec![4, 5, 6]);

    let stream1 = producer1.produce();
    let stream2 = producer2.produce();

    let result1: Vec<i32> = stream1.map(|r| r.unwrap()).collect().await;
    let result2: Vec<i32> = stream2.map(|r| r.unwrap()).collect().await;

    assert_eq!(result1, vec![1, 2, 3]);
    assert_eq!(result2, vec![4, 5, 6]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut producer = VecProducer::new(vec![1, 2, 3])
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      producer.create_error_context(None),
      producer.component_info(),
    );

    assert_eq!(producer.handle_error(error), ErrorStrategy::Skip);
  }
}
