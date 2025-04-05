use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, stream};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use std::time::Duration;

#[derive(Debug)]
pub enum TimeoutError {
  StreamError(String),
}

impl fmt::Display for TimeoutError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      TimeoutError::StreamError(msg) => write!(f, "Stream error: {}", msg),
    }
  }
}

impl StdError for TimeoutError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    None
  }
}

pub struct TimeoutProducer {
  delay: Duration,
  message: String,
}

impl TimeoutProducer {
  pub fn new(delay: Duration, message: impl Into<String>) -> Self {
    Self {
      delay,
      message: message.into(),
    }
  }
}

impl Error for TimeoutProducer {
  type Error = TimeoutError;
}

impl Output for TimeoutProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, TimeoutError>> + Send>>;
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
}
