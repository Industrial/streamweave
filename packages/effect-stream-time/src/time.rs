use crate::error::TimeStreamError;
use chrono::{DateTime, Duration, Utc};
use effect_stream::{EffectError, EffectResult, EffectStream};
use std::future::Future;
use std::pin::Pin;
use tokio::time::sleep;

/// A stream that emits values at regular time intervals
pub struct TimeStream<T> {
  interval: Duration,
  start_time: DateTime<Utc>,
  current_time: DateTime<Utc>,
  value: T,
}

impl<T> TimeStream<T>
where
  T: Clone + Send + Sync + 'static,
{
  /// Create a new TimeStream that emits the given value at regular intervals
  pub fn new(interval: Duration, value: T) -> Self {
    let now = Utc::now();
    Self {
      interval,
      start_time: now,
      current_time: now,
      value,
    }
  }

  /// Convert this TimeStream into an EffectStream
  pub fn into_stream(self) -> EffectStream<T, TimeStreamError> {
    let stream = EffectStream::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      let mut current_time = self.current_time;
      let interval = self.interval;
      let value = self.value;

      loop {
        // Calculate next interval
        current_time = current_time + interval;
        let now = Utc::now();
        if current_time <= now {
          // We're behind schedule, skip to next interval
          current_time = now + interval;
        }

        // Wait until next interval
        let duration = current_time - now;
        sleep(duration.to_std().unwrap()).await;

        // Push value to stream
        if let Err(e) = stream.push(value.clone()).await {
          eprintln!("Error pushing to stream: {:?}", e);
          break;
        }
      }

      // Close stream when done
      if let Err(e) = stream.close().await {
        eprintln!("Error closing stream: {:?}", e);
      }
    });

    stream_clone
  }
}

/// A trait for types that can produce a TimeStream
pub trait TimeStreamSource<T> {
  type Stream: Future<Output = EffectResult<EffectStream<T, TimeStreamError>, TimeStreamError>>
    + Send
    + 'static;

  /// Create a new TimeStream
  fn source(&self) -> Self::Stream;
}

/// A simple test source that produces a TimeStream
pub struct TestTimeSource<T> {
  interval: Duration,
  value: T,
}

impl<T> TestTimeSource<T> {
  /// Create a new TestTimeSource
  pub fn new(interval: Duration, value: T) -> Self {
    Self { interval, value }
  }
}

impl<T> TimeStreamSource<T> for TestTimeSource<T>
where
  T: Clone + Send + Sync + 'static,
{
  type Stream = Pin<
    Box<
      dyn Future<Output = EffectResult<EffectStream<T, TimeStreamError>, TimeStreamError>>
        + Send
        + 'static,
    >,
  >;

  fn source(&self) -> Self::Stream {
    let stream = TimeStream::new(self.interval, self.value.clone());
    Box::pin(async move { Ok(stream.into_stream()) })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration as StdDuration;

  #[tokio::test]
  async fn test_time_stream() {
    let source = TestTimeSource::new(Duration::milliseconds(100), 42);
    let stream = source.source().await.unwrap();

    // Read first value
    let value = stream.next().await.unwrap().unwrap();
    assert_eq!(value, 42);

    // Read second value
    let value = stream.next().await.unwrap().unwrap();
    assert_eq!(value, 42);

    // Close stream
    stream.close().await.unwrap();
  }

  #[tokio::test]
  async fn test_zero_interval() {
    let source = TestTimeSource::new(Duration::milliseconds(0), 42);
    let result = source.source().await;
    assert!(matches!(result, Err(TimeStreamError::ZeroInterval)));
  }
}
