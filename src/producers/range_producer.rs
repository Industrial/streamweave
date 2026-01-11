//! Range producer for generating numeric ranges as streams.
//!
//! This module provides [`RangeProducer`], a producer that generates a sequence of
//! integers from a start value (inclusive) to an end value (exclusive) with a given
//! step size. It's useful for generating test data, iterating over numeric ranges,
//! or creating sequences for processing.
//!
//! # Overview
//!
//! [`RangeProducer`] generates a stream of `i32` values based on a start, end, and
//! step configuration. It supports both ascending (positive step) and descending
//! (negative step) ranges, making it flexible for various use cases.
//!
//! # Key Concepts
//!
//! - **Numeric Ranges**: Generates sequences of integers
//! - **Configurable Step**: Supports positive (ascending) and negative (descending) steps
//! - **Inclusive Start, Exclusive End**: Start is included, end is excluded (like Rust ranges)
//! - **Type Conversion**: Converts from `i64` range parameters to `i32` output (clamps to i32 bounds)
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`RangeProducer`]**: Producer that generates integer ranges
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::producers::RangeProducer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a producer that generates 1, 2, 3, 4, 5
//! let producer = RangeProducer::new(1, 6, 1);
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
//! ## Descending Range
//!
//! ```rust
//! use streamweave::producers::RangeProducer;
//!
//! // Create a producer that generates 10, 9, 8, 7, 6 (descending)
//! let producer = RangeProducer::new(10, 5, -1);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::producers::RangeProducer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a producer with error handling strategy
//! let producer = RangeProducer::new(0, 100, 2)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("even-numbers".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **i32 Output**: Outputs `i32` values for compatibility with common integer types
//! - **i64 Parameters**: Uses `i64` for range parameters to support larger ranges,
//!   but clamps to `i32` bounds in the output
//! - **Inclusive/Exclusive**: Follows Rust's range convention (start inclusive, end exclusive)
//! - **Step Validation**: Panics on zero step to prevent infinite loops
//! - **Pre-computation**: Generates the entire range upfront for simplicity
//!
//! # Integration with StreamWeave
//!
//! [`RangeProducer`] implements the [`Producer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`ProducerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use futures::{Stream, stream};
use std::pin::Pin;

/// A producer that generates a range of numbers.
///
/// This producer yields numbers from `start` (inclusive) to `end` (exclusive)
/// with a given `step` size. It's useful for generating test data or iterating
/// over numeric ranges.
///
/// # Example
///
/// ```rust,no_run
/// use crate::producers::RangeProducer;
///
/// // Produces: 1, 2, 3, 4, 5
/// let producer = RangeProducer::new(1, 6, 1);
///
/// // Produces: 0, 2, 4, 6, 8
/// let producer = RangeProducer::new(0, 10, 2);
///
/// // Produces: 10, 9, 8, 7, 6
/// let producer = RangeProducer::new(10, 5, -1);
/// ```
#[derive(Clone)]
pub struct RangeProducer {
  /// The starting value (inclusive).
  pub start: i64,
  /// The ending value (exclusive).
  pub end: i64,
  /// The step size (can be negative for descending ranges).
  pub step: i64,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<i32>,
}

impl RangeProducer {
  /// Creates a new `RangeProducer` with the given range parameters.
  ///
  /// # Arguments
  ///
  /// * `start` - The starting value (inclusive).
  /// * `end` - The ending value (exclusive).
  /// * `step` - The step size. Must be non-zero. Use negative values for descending ranges.
  ///
  /// # Panics
  ///
  /// Panics if `step` is zero.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use crate::producers::RangeProducer;
  ///
  /// // Produces: 1, 2, 3, 4, 5
  /// let producer = RangeProducer::new(1, 6, 1);
  /// ```
  pub fn new(start: i64, end: i64, step: i64) -> Self {
    assert_ne!(step, 0, "step must be non-zero");
    Self {
      start,
      end,
      step,
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<i32>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Output for RangeProducer {
  type Output = i32;
  type OutputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
}

impl Producer for RangeProducer {
  type OutputPorts = (i32,);

  fn produce(&mut self) -> Self::OutputStream {
    let start = self.start;
    let end = self.end;
    let step = self.step;

    // Generate the range items
    let mut items = Vec::new();
    let mut current = start;

    if step > 0 {
      while current < end {
        // Only add if it's within i32 bounds
        if current >= i32::MIN as i64 && current <= i32::MAX as i64 {
          items.push(current as i32);
        }
        current += step;
      }
    } else {
      // For negative steps, iterate backwards
      while current > end {
        // Only add if it's within i32 bounds
        if current >= i32::MIN as i64 && current <= i32::MAX as i64 {
          items.push(current as i32);
        }
        current += step;
      }
    }

    Box::pin(stream::iter(items))
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
      ErrorStrategy::Custom(ref handler) => handler(error),
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
        .unwrap_or_else(|| "range_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "range_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
