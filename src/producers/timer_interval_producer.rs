//! Timer interval producer for generating periodic timestamp events.
//!
//! This module provides [`TimerIntervalProducer`], a producer that emits timestamp
//! events at regular intervals. It's useful for generating periodic events, heartbeats,
//! or time-based triggers in streaming pipelines.
//!
//! # Overview
//!
//! [`TimerIntervalProducer`] generates a continuous stream of `SystemTime` timestamps
//! at the specified interval. Each emitted item represents the current system time at
//! the moment the interval elapses. The producer runs indefinitely until the stream
//! is dropped or cancelled.
//!
//! # Key Concepts
//!
//! - **Periodic Events**: Emits events at fixed intervals
//! - **Timestamp Output**: Each event is a `SystemTime` timestamp
//! - **Infinite Stream**: Runs indefinitely until cancelled
//! - **Async Timing**: Uses Tokio's async interval timer for precise timing
//! - **Error Handling**: Configurable error strategies (though errors are rare)
//!
//! # Core Types
//!
//! - **[`TimerIntervalProducer`]**: Producer that emits timestamps at intervals
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::producers::TimerIntervalProducer;
//! use streamweave::PipelineBuilder;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that emits every second
//! let producer = TimerIntervalProducer::new(Duration::from_secs(1));
//!
//! // Use in a pipeline
//! let pipeline = PipelineBuilder::new()
//!     .producer(producer)
//!     .transformer(/* ... */)
//!     .consumer(/* ... */);
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::producers::TimerIntervalProducer;
//! use streamweave::ErrorStrategy;
//! use std::time::Duration;
//!
//! // Create a producer with error handling strategy
//! let producer = TimerIntervalProducer::new(Duration::from_millis(100))
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("heartbeat".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **SystemTime Output**: Emits `SystemTime` rather than a custom timestamp type
//!   for maximum compatibility and standard time representation
//! - **Infinite Stream**: Runs indefinitely to support long-running processes that
//!   need continuous periodic events
//! - **Async Interval**: Uses Tokio's `interval` for efficient, async-compatible timing
//! - **Precise Timing**: Tokio's interval timer provides accurate timing even under load
//!
//! # Integration with StreamWeave
//!
//! [`TimerIntervalProducer`] implements the [`Producer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ProducerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use futures::Stream;
use std::pin::Pin;
use std::time::Duration;
use tokio::time;

/// A producer that emits events at regular intervals.
///
/// This producer generates timestamp events at the specified interval duration.
pub struct TimerIntervalProducer {
  /// The interval duration between events
  pub interval: Duration,
  /// Configuration for the producer
  pub config: ProducerConfig<std::time::SystemTime>,
}

impl TimerIntervalProducer {
  /// Creates a new `TimerIntervalProducer` with the given interval.
  ///
  /// # Arguments
  ///
  /// * `interval` - The duration between emitted events
  pub fn new(interval: Duration) -> Self {
    Self {
      interval,
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<std::time::SystemTime>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Output for TimerIntervalProducer {
  type Output = std::time::SystemTime;
  type OutputStream = Pin<Box<dyn Stream<Item = std::time::SystemTime> + Send>>;
}

impl Producer for TimerIntervalProducer {
  type OutputPorts = (std::time::SystemTime,);

  fn produce(&mut self) -> Self::OutputStream {
    let interval = self.interval;
    Box::pin(async_stream::stream! {
      let mut interval_stream = time::interval(interval);
      loop {
        interval_stream.tick().await;
        yield std::time::SystemTime::now();
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<std::time::SystemTime>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<std::time::SystemTime> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<std::time::SystemTime> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<std::time::SystemTime>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(
    &self,
    item: Option<std::time::SystemTime>,
  ) -> ErrorContext<std::time::SystemTime> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "timer_interval_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "timer_interval_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
