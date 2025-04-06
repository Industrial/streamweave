use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  error::Error,
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, stream};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub struct TimeoutProducer {
  delay: Duration,
  message: String,
  config: ProducerConfig,
}

impl TimeoutProducer {
  pub fn new(delay: Duration, message: impl Into<String>) -> Self {
    Self {
      delay,
      message: message.into(),
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

impl Error for TimeoutProducer {
  type Error = StreamError;
}

impl Output for TimeoutProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

impl Producer for TimeoutProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let message = self.message.clone();
    let delay = self.delay;

    Box::pin(stream::once(async move {
      tokio::time::sleep(delay).await;
      Ok(message)
    }))
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
        .unwrap_or_else(|| "timeout_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use std::time::{Duration, Instant};

  #[tokio::test]
  async fn test_timeout_producer() {
    let mut producer = TimeoutProducer::new(Duration::from_millis(100), "test message");
    let start = Instant::now();
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
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
    let result1: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1, vec!["test"]);

    // Second call
    let stream = producer.produce();
    let result2: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2, vec!["test"]);
  }

  #[tokio::test]
  async fn test_timing_accuracy() {
    let duration = Duration::from_millis(200);
    let mut producer = TimeoutProducer::new(duration, "test");

    let start = Instant::now();
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    let elapsed = start.elapsed();

    assert_eq!(result, vec!["test"]);
    // Check that timing is within reasonable bounds
    assert!(elapsed >= duration);
    assert!(elapsed < duration + Duration::from_millis(50));
  }

  #[tokio::test]
  async fn test_different_messages() {
    let mut producer = TimeoutProducer::new(Duration::from_millis(10), "message 1");
    let result1: Vec<String> = producer.produce().map(|r| r.unwrap()).collect().await;
    assert_eq!(result1, vec!["message 1"]);

    let mut producer = TimeoutProducer::new(Duration::from_millis(10), "message 2");
    let result2: Vec<String> = producer.produce().map(|r| r.unwrap()).collect().await;
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

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      producer.create_error_context(None),
      producer.component_info(),
    );

    assert_eq!(producer.handle_error(error), ErrorStrategy::Skip);
  }
}
