use crate::error::TimeStreamError;
use chrono::{DateTime, Utc};
use effect_core::EffectError;
use effect_stream::{EffectResult, EffectStream};
use std::time::Duration;
use tokio::time::sleep;

/// A stream that emits a single value after a specified duration
pub struct TimerStream {
  stream: EffectStream<DateTime<Utc>, TimeStreamError>,
}

impl TimerStream {
  /// Create a new timer stream that emits a value after the specified duration
  pub fn new(duration: Duration) -> Self {
    if duration.is_zero() {
      panic!("Timer duration cannot be zero");
    }

    let stream = EffectStream::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      tokio::time::sleep(duration).await;
      stream_clone.push(Utc::now()).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    Self { stream }
  }

  /// Get the next value from the stream
  pub async fn next(&self) -> EffectResult<Option<DateTime<Utc>>, TimeStreamError> {
    self.stream.next().await
  }

  /// Close the stream
  pub async fn close(&self) -> EffectResult<(), TimeStreamError> {
    self.stream.close().await
  }
}

impl Clone for TimerStream {
  fn clone(&self) -> Self {
    Self {
      stream: self.stream.clone(),
    }
  }
}
