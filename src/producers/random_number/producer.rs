use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use super::random_number_producer::RandomNumberProducer;
use crate::producer::{Producer, ProducerConfig};
use futures::{StreamExt, stream};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tokio::time;

impl Producer for RandomNumberProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let range = self.range.clone();
    let interval_duration = self.interval;
    let count = self.count;
    let rng = StdRng::from_entropy();

    let initial_state = (rng, range, time::interval(interval_duration));

    let stream = stream::unfold(
      initial_state,
      move |(mut rng, range, mut interval)| async move {
        interval.tick().await;
        let number = rng.gen_range(range.clone());
        let new_seed = std::time::SystemTime::now()
          .duration_since(std::time::UNIX_EPOCH)
          .unwrap()
          .as_nanos() as u64;
        rng = StdRng::seed_from_u64(new_seed);
        Some((number, (rng, range, interval)))
      },
    );

    match count {
      Some(n) => Box::pin(stream.take(n)),
      None => Box::pin(stream),
    }
  }

  fn set_config_impl(&mut self, config: ProducerConfig<i32>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<i32> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<i32> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<i32>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<i32>) -> ErrorContext<i32> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "random_number_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
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
  use std::ops::Range;
  use std::time::Duration;

  #[tokio::test]
  async fn test_random_number_producer() {
    let mut producer = RandomNumberProducer::new(Range { start: 0, end: 100 });
    let stream = producer.produce();
    let result: Vec<i32> = stream.take(2).collect().await;

    assert_eq!(result.len(), 2);
    assert_ne!(result[0], result[1], "Random numbers should be different");
  }

  #[tokio::test]
  async fn test_random_number_timing() {
    let mut producer = RandomNumberProducer::new(Range { start: 0, end: 100 });
    let start = time::Instant::now();
    let stream = producer.produce();
    let result: Vec<i32> = stream.take(4).collect().await;
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

    // Verify we got some random numbers (they might repeat, which is normal for random numbers)
    assert!(
      !result.is_empty(),
      "Should have produced some random numbers"
    );
  }

  #[tokio::test]
  async fn test_multiple_produces() {
    let mut producer = RandomNumberProducer::with_count(Range { start: 0, end: 100 }, 2);

    // First call
    let stream = producer.produce();
    let result1: Vec<i32> = stream.collect().await;
    assert_eq!(result1.len(), 2);

    // Second call
    let stream = producer.produce();
    let result2: Vec<i32> = stream.collect().await;
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
    let result: Vec<i32> = stream.take(10).collect().await;

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
    let result: Vec<i32> = stream.collect().await;
    assert_eq!(result.len(), 3);
  }

  #[test]
  fn test_error_handling_strategies() {
    let producer = RandomNumberProducer::new(Range { start: 0, end: 100 })
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
        component_type: "RandomNumberProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "RandomNumberProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
