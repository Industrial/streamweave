use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::traits::{
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, stream};
use std::pin::Pin;

pub struct VecProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  data: Vec<T>,
  config: ProducerConfig<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> VecProducer<T> {
  pub fn new(data: Vec<T>) -> Self {
    Self {
      data,
      config: ProducerConfig::default(),
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

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Output for VecProducer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Producer for VecProducer<T> {
  fn produce(&mut self) -> Self::OutputStream {
    let stream = stream::iter(self.data.clone().into_iter());
    Box::pin(stream)
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
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
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "vec_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
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
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, data);
  }

  #[tokio::test]
  async fn test_vec_producer_empty() {
    let mut producer = VecProducer::<String>::new(vec![]);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert_eq!(result, Vec::<String>::new());
  }

  #[tokio::test]
  async fn test_vec_producer_multiple_calls() {
    let data = vec!["test1".to_string(), "test2".to_string()];
    let mut producer = VecProducer::new(data.clone());

    // First call
    let stream = producer.produce();
    let result1: Vec<String> = stream.collect().await;
    assert_eq!(result1, data);

    // Second call
    let stream = producer.produce();
    let result2: Vec<String> = stream.collect().await;
    assert_eq!(result2, data);
  }

  #[tokio::test]
  async fn test_vec_producer_clone_independence() {
    let mut producer1 = VecProducer::new(vec![1, 2, 3]);
    let mut producer2 = VecProducer::new(vec![4, 5, 6]);

    let stream1 = producer1.produce();
    let stream2 = producer2.produce();

    let result1: Vec<i32> = stream1.collect().await;
    let result2: Vec<i32> = stream2.collect().await;

    assert_eq!(result1, vec![1, 2, 3]);
    assert_eq!(result2, vec![4, 5, 6]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let producer = VecProducer::new(vec![1, 2, 3])
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "VecProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "VecProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
