use crate::error::TimeStreamError;
use chrono::{DateTime, Utc};
use effect_core::EffectError;
use effect_stream::{EffectResult, EffectStream};
use std::time::Duration;
use tokio::time::interval;

/// A stream that emits values at regular intervals
pub struct IntervalStream {
  stream: EffectStream<DateTime<Utc>, TimeStreamError>,
}

impl IntervalStream {
  /// Create a new interval stream with the given duration
  pub fn new(duration: Duration) -> EffectResult<Self, TimeStreamError> {
    if duration.is_zero() {
      return Err(TimeStreamError::InvalidInterval("Duration cannot be zero".to_string()).into());
    }

    let stream = EffectStream::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      let mut interval = interval(duration);
      loop {
        interval.tick().await;
        if let Err(e) = stream_clone.push(Utc::now()).await {
          break;
        }
      }
    });

    Ok(Self { stream })
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

impl Clone for IntervalStream {
  fn clone(&self) -> Self {
    Self {
      stream: self.stream.clone(),
    }
  }
}
