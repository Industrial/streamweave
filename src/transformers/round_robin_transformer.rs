//! Round-robin transformer for load balancing stream items.
//!
//! This module provides [`RoundRobinTransformer<T>`], a transformer that distributes
//! stream items to multiple consumers in round-robin fashion. Each item is sent to
//! exactly one consumer, cycling through consumers in order. This is useful for load
//! balancing, work distribution, and parallel processing.
//!
//! # Overview
//!
//! [`RoundRobinTransformer`] implements a round-robin distribution pattern where items
//! are distributed evenly across multiple consumers. Unlike broadcast transformers that
//! clone items to all consumers, round-robin sends each item to exactly one consumer,
//! cycling through consumers in order (0, 1, 2, ..., 0, 1, 2, ...).
//!
//! # Key Concepts
//!
//! - **Round-Robin Distribution**: Items are distributed evenly across consumers
//! - **Single Destination**: Each item goes to exactly one consumer (no cloning)
//! - **Load Balancing**: Distributes work evenly across multiple workers
//! - **Consumer Indexing**: Output includes consumer index for routing
//! - **Thread-Safe**: Uses atomic operations for thread-safe distribution
//!
//! # Core Types
//!
//! - **[`RoundRobinTransformer<T>`]**: Transformer that distributes items in round-robin fashion
//! - **[`RoundRobinConfig`]**: Configuration for round-robin behavior
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::RoundRobinTransformer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that distributes to 3 consumers
//! let transformer = RoundRobinTransformer::<i32>::new(3);
//!
//! // Input: [1, 2, 3, 4, 5, 6]
//! // Output: [(0, 1), (1, 2), (2, 3), (0, 4), (1, 5), (2, 6)]
//! //         ^consumer index, item
//! # Ok(())
//! # }
//! ```
//!
//! ## With Configuration
//!
//! ```rust
//! use streamweave::transformers::{RoundRobinTransformer, RoundRobinConfig};
//!
//! // Create a transformer with custom configuration
//! let transformer = RoundRobinTransformer::with_config(
//!     RoundRobinConfig::new(5)
//!         .with_include_index(true)
//! );
//! ```
//!
//! # Design Decisions
//!
//! - **Atomic Counter**: Uses atomic operations for thread-safe round-robin distribution
//! - **Indexed Output**: Output includes consumer index for downstream routing
//! - **No Cloning**: Unlike broadcast, items are not cloned (each item to one consumer)
//! - **Even Distribution**: Ensures even load distribution across consumers
//! - **Generic Type**: Generic over item type for maximum flexibility
//!
//! # Integration with StreamWeave
//!
//! [`RoundRobinTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline or graph. It supports the standard error handling
//! strategies and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Configuration for round-robin distribution behavior.
#[derive(Debug, Clone)]
pub struct RoundRobinConfig {
  /// Number of consumers to distribute to.
  pub num_consumers: usize,
  /// Whether to include consumer index with each element.
  pub include_index: bool,
}

impl Default for RoundRobinConfig {
  fn default() -> Self {
    Self {
      num_consumers: 2,
      include_index: false,
    }
  }
}

impl RoundRobinConfig {
  /// Creates a new RoundRobinConfig with the specified number of consumers.
  #[must_use]
  pub fn new(num_consumers: usize) -> Self {
    Self {
      num_consumers,
      ..Default::default()
    }
  }

  /// Sets whether to include consumer index with each element.
  #[must_use]
  pub fn with_include_index(mut self, include: bool) -> Self {
    self.include_index = include;
    self
  }
}

/// A transformer that distributes elements to consumers in round-robin fashion.
///
/// This implements load balancing where each element is sent to exactly one
/// consumer, cycling through consumers in order. This is useful for:
/// - Distributing work across multiple workers
/// - Load balancing parallel processing
/// - Partitioning data into equal-sized batches
///
/// Unlike broadcast, round-robin does NOT clone elements - each element
/// goes to exactly one consumer.
pub struct RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Configuration for the transformer.
  pub config: TransformerConfig<T>,
  /// Round-robin specific configuration.
  pub rr_config: RoundRobinConfig,
  /// Current consumer index (atomic for thread safety).
  pub current_index: Arc<AtomicUsize>,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> Clone for RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      rr_config: self.rr_config.clone(),
      current_index: Arc::clone(&self.current_index),
      _phantom: PhantomData,
    }
  }
}

impl<T> RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new RoundRobinTransformer with the specified number of consumers.
  #[must_use]
  pub fn new(num_consumers: usize) -> Self {
    Self {
      config: TransformerConfig::default(),
      rr_config: RoundRobinConfig::new(num_consumers),
      current_index: Arc::new(AtomicUsize::new(0)),
      _phantom: PhantomData,
    }
  }

  /// Creates a new RoundRobinTransformer with custom configuration.
  #[must_use]
  pub fn with_config(rr_config: RoundRobinConfig) -> Self {
    Self {
      config: TransformerConfig::default(),
      rr_config,
      current_index: Arc::new(AtomicUsize::new(0)),
      _phantom: PhantomData,
    }
  }

  /// Sets the error strategy for the transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  /// Sets the name for the transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }

  /// Returns the number of consumers.
  #[must_use]
  pub fn num_consumers(&self) -> usize {
    self.rr_config.num_consumers
  }

  /// Gets the next consumer index and advances the counter.
  pub fn next_consumer(&self) -> usize {
    let num = self.rr_config.num_consumers;
    if num == 0 {
      return 0;
    }
    self.current_index.fetch_add(1, Ordering::SeqCst) % num
  }

  /// Resets the counter to start from consumer 0.
  pub fn reset(&self) {
    self.current_index.store(0, Ordering::SeqCst);
  }

  /// Returns the current index without advancing.
  #[must_use]
  pub fn current_consumer(&self) -> usize {
    let num = self.rr_config.num_consumers;
    if num == 0 {
      return 0;
    }
    self.current_index.load(Ordering::SeqCst) % num
  }
}

impl<T> Default for RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new(2)
  }
}

impl<T> Input for RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Self::Input> + Send>>;
}

impl<T> Output for RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Output is a tuple of (consumer_index, element).
  /// This allows downstream processing to route elements to the correct consumer.
  type Output = (usize, T);
  type OutputStream = Pin<Box<dyn Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl<T> Transformer for RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = ((usize, T),);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let num_consumers = self.rr_config.num_consumers;
    let current_index = Arc::clone(&self.current_index);

    Box::pin(input.map(move |item| {
      let index = if num_consumers == 0 {
        0
      } else {
        current_index.fetch_add(1, Ordering::SeqCst) % num_consumers
      };
      (index, item)
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "round_robin_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "round_robin_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
