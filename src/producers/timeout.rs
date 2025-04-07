use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, stream};
use std::pin::Pin;
use std::time::Duration;
use tokio::time;

pub struct TimeoutProducer {
  delay: Duration,
  message: String,
  config: ProducerConfig<String>,
}

impl TimeoutProducer {
  pub fn new(delay: Duration, message: impl Into<String>) -> Self {
    Self {
      delay,
      message: message.into(),
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Output for TimeoutProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Producer for TimeoutProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let message = self.message.clone();
    let delay = self.delay;

    Box::pin(stream::once(async move {
      time::sleep(delay).await;
      message
    }))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "timeout_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_timeout_producer() {
    let mut producer = TimeoutProducer::new(Duration::from_millis(100), "test message");
    let start = time::Instant::now();
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    let elapsed = start.elapsed();

    assert_eq!(result, vec!["test message"]);
    assert!(elapsed >= Duration::from_millis(100));
    assert!(elapsed < Duration::from_millis(150)); // Add some buffer
  }

  #[tokio::test]
  async fn test_multiple_produces() {
    let mut producer = TimeoutProducer::new(Duration::from_millis(10), "test");

    // First call
    let stream = producer.produce();
    let result1: Vec<String> = stream.collect().await;
    assert_eq!(result1, vec!["test"]);

    // Second call
    let stream = producer.produce();
    let result2: Vec<String> = stream.collect().await;
    assert_eq!(result2, vec!["test"]);
  }

  #[tokio::test]
  async fn test_timing_accuracy() {
    let duration = Duration::from_millis(200);
    let mut producer = TimeoutProducer::new(duration, "test");

    let start = time::Instant::now();
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    let elapsed = start.elapsed();

    assert_eq!(result, vec!["test"]);
    // Check that timing is within reasonable bounds
    assert!(elapsed >= duration);
    assert!(elapsed < duration + Duration::from_millis(50));
  }

  #[tokio::test]
  async fn test_different_messages() {
    let mut producer = TimeoutProducer::new(Duration::from_millis(10), "message 1");
    let result1: Vec<String> = producer.produce().collect().await;
    assert_eq!(result1, vec!["message 1"]);

    let mut producer = TimeoutProducer::new(Duration::from_millis(10), "message 2");
    let result2: Vec<String> = producer.produce().collect().await;
    assert_eq!(result2, vec!["message 2"]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut producer = TimeoutProducer::new(Duration::from_millis(100), "test message")
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
        stage: PipelineStage::Producer,
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TimeoutProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
