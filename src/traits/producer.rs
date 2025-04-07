use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::output::Output;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct ProducerConfig<T: Clone> {
  pub error_strategy: ErrorStrategy<T>,
  pub name: Option<String>,
}

impl<T: Clone> Default for ProducerConfig<T> {
  fn default() -> Self {
    Self {
      error_strategy: ErrorStrategy::Stop,
      name: None,
    }
  }
}

impl<T: Clone> ProducerConfig<T> {
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.name = Some(name);
    self
  }

  pub fn error_strategy(&self) -> ErrorStrategy<T> {
    self.error_strategy.clone()
  }

  pub fn name(&self) -> Option<String> {
    self.name.clone()
  }
}

#[async_trait]
pub trait Producer: Output
where
  Self::Output: Clone,
{
  fn produce(&mut self) -> Self::OutputStream;

  fn with_config(&self, config: ProducerConfig<Self::Output>) -> Self
  where
    Self: Sized + Clone,
  {
    let mut this = self.clone();
    this.set_config(config);
    this
  }

  fn set_config(&mut self, config: ProducerConfig<Self::Output>) {
    self.set_config_impl(config);
  }

  fn config(&self) -> &ProducerConfig<Self::Output> {
    self.get_config_impl()
  }

  fn config_mut(&mut self) -> &mut ProducerConfig<Self::Output> {
    self.get_config_mut_impl()
  }

  fn with_name(mut self, name: String) -> Self
  where
    Self: Sized,
  {
    let config = self.get_config_impl().clone();
    self.set_config(ProducerConfig {
      error_strategy: config.error_strategy,
      name: Some(name),
    });
    self
  }

  fn handle_error(&self, error: &StreamError<Self::Output>) -> ErrorAction {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Output>) -> ErrorContext<Self::Output> {
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
        .unwrap_or_else(|| "producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }

  // These methods need to be implemented by each producer
  fn set_config_impl(&mut self, config: ProducerConfig<Self::Output>);
  fn get_config_impl(&self) -> &ProducerConfig<Self::Output>;
  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Self::Output>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::{ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError};
  use futures::StreamExt;
  use std::fmt;
  use std::pin::Pin;
  use std::sync::{Arc, Mutex};
  use tokio_stream::Stream;

  // Test error type
  #[derive(Debug)]
  struct TestError(String);

  impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "Test error: {}", self.0)
    }
  }

  // Test producer that yields items from a vector
  #[derive(Clone)]
  struct TestProducer<T: Clone> {
    items: Vec<T>,
    config: ProducerConfig<T>,
  }

  impl<T: Clone> TestProducer<T> {
    fn new(items: Vec<T>) -> Self {
      Self {
        items,
        config: ProducerConfig::default(),
      }
    }
  }

  impl<T: Clone + Send + 'static> Output for TestProducer<T> {
    type Output = T;
    type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
  }

  #[async_trait]
  impl<T: Clone + Send + 'static> Producer for TestProducer<T> {
    fn produce(&mut self) -> Self::OutputStream {
      let items = self.items.clone();
      Box::pin(futures::stream::iter(items))
    }

    fn set_config_impl(&mut self, config: ProducerConfig<Self::Output>) {
      self.config = config;
    }

    fn get_config_impl(&self) -> &ProducerConfig<Self::Output> {
      &self.config
    }

    fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Self::Output> {
      &mut self.config
    }
  }

  #[tokio::test]
  async fn test_producer() {
    let mut producer = TestProducer::new(vec![1, 2, 3]);
    let stream = producer.produce();
    let result: Vec<i32> = stream.collect().await;
    assert_eq!(result, vec![1, 2, 3]);
  }

  #[test]
  fn test_producer_config() {
    let mut producer = TestProducer::new(vec![1, 2, 3])
      .with_name("test_producer".to_string())
      .with_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Skip));

    assert_eq!(producer.config().name(), Some("test_producer".to_string()));
    assert!(matches!(
      producer.config().error_strategy(),
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_producer_error_handling() {
    let mut producer = TestProducer::new(vec![1, 2, 3])
      .with_config(ProducerConfig::default().with_error_strategy(ErrorStrategy::Skip));

    let error = StreamError {
      source: Box::new(TestError("test error".to_string())),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestProducer".to_string(),
      },
      retries: 0,
    };

    assert!(matches!(producer.handle_error(&error), ErrorAction::Skip));
  }
}
