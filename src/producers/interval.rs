use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum IntervalError {
  StreamError(String),
}

impl fmt::Display for IntervalError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      IntervalError::StreamError(msg) => write!(f, "Stream error: {}", msg),
    }
  }
}

impl StdError for IntervalError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    None
  }
}

pub struct IntervalProducer {
  interval: Duration,
  count: Option<usize>,
}

impl IntervalProducer {
  pub fn new(interval: Duration) -> Self {
    Self {
      interval,
      count: None,
    }
  }

  pub fn with_count(interval: Duration, count: usize) -> Self {
    Self {
      interval,
      count: Some(count),
    }
  }
}

impl Error for IntervalProducer {
  type Error = IntervalError;
}

impl Output for IntervalProducer {
  type Output = Instant;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, IntervalError>> + Send>>;
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
}
