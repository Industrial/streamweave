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
use std::time::{Duration, Instant};

pub struct IntervalProducer {
  interval: Duration,
  count: Option<usize>,
  config: ProducerConfig,
}

impl IntervalProducer {
  pub fn new(interval: Duration) -> Self {
    Self {
      interval,
      count: None,
      config: ProducerConfig::default(),
    }
  }

  pub fn with_count(interval: Duration, count: usize) -> Self {
    Self {
      interval,
      count: Some(count),
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

impl Error for IntervalProducer {
  type Error = StreamError;
}

impl Output for IntervalProducer {
  type Output = Instant;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

impl Producer for IntervalProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let interval = self.interval;
    let count = self.count;

    // Create a stream from the interval's ticks
    let mut interval = tokio::time::interval(interval);
    let stream =
      futures::stream::poll_fn(move |cx| interval.poll_tick(cx).map(|_| Some(Ok(Instant::now()))));

    match count {
      Some(n) => Box::pin(stream.take(n)),
      None => Box::pin(stream),
    }
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
        .unwrap_or_else(|| "interval_producer".to_string()),
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
  async fn test_interval_producer() {
    let mut producer = IntervalProducer::new(Duration::from_millis(100));
    let stream = producer.produce();
    let items: Vec<Instant> = stream.map(|r| r.unwrap()).take(3).collect().await;

    assert_eq!(items.len(), 3);

    // Check that the timestamps are sequential and roughly 100ms apart
    for i in 1..items.len() {
      let duration = items[i].duration_since(items[i - 1]);
      assert!(
        duration >= Duration::from_millis(90) && duration <= Duration::from_millis(110),
        "Expected interval between {} and {} to be ~100ms but was {:?}",
        i - 1,
        i,
        duration
      );
    }
  }

  #[tokio::test]
  async fn test_interval_timing() {
    let interval = Duration::from_millis(100);
    let mut producer = IntervalProducer::new(interval);

    let start = Instant::now();
    let stream = producer.produce();
    // Collect 4 items to ensure we're measuring multiple intervals
    let result: Vec<Instant> = stream.map(|r| r.unwrap()).take(4).collect().await;
    let elapsed = start.elapsed();

    // We expect approximately 300ms (3 intervals) for 4 items
    // First item comes immediately, then wait 100ms each for the next 3
    assert_eq!(result.len(), 4);
    // Add some tolerance to account for system scheduling
    assert!(
      elapsed >= Duration::from_millis(250),
      "Elapsed time was {:?}",
      elapsed
    );
    assert!(
      elapsed <= Duration::from_millis(400),
      "Elapsed time was {:?}",
      elapsed
    );
  }

  #[tokio::test]
  async fn test_multiple_produces() {
    let mut producer = IntervalProducer::with_count(Duration::from_millis(10), 1);

    // First call
    let stream = producer.produce();
    let result1: Vec<Instant> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1.len(), 1);

    // Second call
    let stream = producer.produce();
    let result2: Vec<Instant> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2.len(), 1);
  }

  #[tokio::test]
  async fn test_with_count() {
    let mut producer = IntervalProducer::with_count(Duration::from_millis(10), 3);
    let stream = producer.produce();
    let result: Vec<Instant> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result.len(), 3);
  }

  #[tokio::test]
  async fn test_infinite_stream() {
    let mut producer = IntervalProducer::new(Duration::from_millis(10));
    let stream = producer.produce();
    let result: Vec<Instant> = stream.map(|r| r.unwrap()).take(5).collect().await;
    assert_eq!(result.len(), 5);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut producer = IntervalProducer::new(Duration::from_millis(100))
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
