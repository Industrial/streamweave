use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  error::Error,
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, StreamExt};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::error::Error as StdError;
use std::fmt;
use std::ops::Range;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

#[derive(Debug)]
pub enum RandomNumberError {
  StreamError(String),
}

impl fmt::Display for RandomNumberError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      RandomNumberError::StreamError(msg) => write!(f, "Stream error: {}", msg),
    }
  }
}

impl StdError for RandomNumberError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    None
  }
}

pub struct RandomNumberProducer {
  range: Range<i32>,
  count: Option<usize>,
  interval: Duration,
  config: ProducerConfig,
}

impl RandomNumberProducer {
  pub fn new(range: Range<i32>) -> Self {
    Self {
      range,
      count: None,
      interval: Duration::from_millis(100),
      config: ProducerConfig::default(),
    }
  }

  pub fn with_count(range: Range<i32>, count: usize) -> Self {
    Self {
      range,
      count: Some(count),
      interval: Duration::from_millis(100),
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

impl Error for RandomNumberProducer {
  type Error = StreamError;
}

impl Output for RandomNumberProducer {
  type Output = i32;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

impl Producer for RandomNumberProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let range = self.range.clone();
    let interval_duration = self.interval;
    let count = self.count;
    let mut rng = StdRng::from_entropy();

    let initial_state = (rng, range, interval(interval_duration));

    let stream = futures::stream::unfold(
      initial_state,
      move |(mut rng, range, mut interval)| async move {
        interval.tick().await;
        let number = rng.gen_range(range.clone());
        Some((Ok(number), (rng, range, interval)))
      },
    );

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
        .unwrap_or_else(|| "random_number_producer".to_string()),
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
  async fn test_random_number_producer() {
    let mut producer = RandomNumberProducer::new(Range { start: 0, end: 100 });
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).take(2).collect().await;

    assert_eq!(result.len(), 2);
    assert_ne!(result[0], result[1], "Random numbers should be different");
  }

  #[tokio::test]
  async fn test_random_number_timing() {
    let mut producer = RandomNumberProducer::new(Range { start: 0, end: 100 });
    let start = Instant::now();
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).take(4).collect().await;
    let elapsed = start.elapsed();

    assert_eq!(result.len(), 4);
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

    // Verify numbers are different
    for i in 0..result.len() - 1 {
      assert_ne!(
        result[i],
        result[i + 1],
        "Consecutive random numbers should be different"
      );
    }
  }

  #[tokio::test]
  async fn test_multiple_produces() {
    let mut producer = RandomNumberProducer::with_count(Range { start: 0, end: 100 }, 2);

    // First call
    let stream = producer.produce();
    let result1: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1.len(), 2);

    // Second call
    let stream = producer.produce();
    let result2: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2.len(), 2);

    // Verify the numbers are likely different between calls
    assert_ne!(
      result1, result2,
      "Different calls should produce different sequences"
    );
  }

  #[tokio::test]
  async fn test_random_number_uniqueness() {
    let mut producer = RandomNumberProducer::new(Range { start: 0, end: 50 });
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).take(10).collect().await;

    // Check that we have some variation in the numbers
    let unique_count = result
      .iter()
      .collect::<std::collections::HashSet<_>>()
      .len();
    assert!(
      unique_count > 5,
      "Expected more unique random numbers in {:?}",
      result
    );
  }

  #[tokio::test]
  async fn test_with_count() {
    let mut producer = RandomNumberProducer::with_count(Range { start: 0, end: 100 }, 3);
    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result.len(), 3);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut producer = RandomNumberProducer::new(Range { start: 0, end: 100 })
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
