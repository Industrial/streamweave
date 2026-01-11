//! # Sample Node
//!
//! Graph node that randomly samples items from streams based on a probability.
//! This module provides [`Sample`], a graph node that passes through each item
//! with a configurable probability, creating a random sample of the input stream.
//! It wraps [`SampleTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`Sample`] is useful for creating random samples from large streams in
//! graph-based pipelines. It allows you to process only a subset of items
//! probabilistically, making it ideal for testing, statistical sampling, and
//! reducing processing load on large datasets.
//!
//! # Key Concepts
//!
//! - **Probabilistic Sampling**: Each item has a configurable probability of
//!   being passed through
//! - **Random Selection**: Uses random number generation for unbiased sampling
//! - **Probability Configuration**: Probability specified as a value between 0.0
//!   and 1.0
//! - **Transformer Wrapper**: Wraps `SampleTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Sample<T>`]**: Node that randomly samples items from a stream
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Sample;
//!
//! // Create a sample node that passes 10% of items (probability 0.1)
//! let sample = Sample::<i32>::new(0.1);
//! ```
//!
//! ## Different Probability Values
//!
//! ```rust
//! use streamweave::graph::nodes::Sample;
//!
//! // 50% probability (half of items)
//! let sample_50 = Sample::<String>::new(0.5);
//!
//! // 1% probability (1 in 100 items)
//! let sample_1 = Sample::<String>::new(0.01);
//!
//! // 100% probability (all items pass through)
//! let sample_all = Sample::<String>::new(1.0);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Sample;
//! use streamweave::ErrorStrategy;
//!
//! // Create a sample node with error handling strategy
//! let sample = Sample::<i32>::new(0.1)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("random-sampler".to_string());
//! ```
//!
//! # Probability Values
//!
//! The probability value must be between 0.0 and 1.0 (inclusive):
//!
//! - **0.0**: No items pass through (empty output)
//! - **0.5**: Half of items pass through (50% sampling)
//! - **1.0**: All items pass through (no filtering)
//!
//! # Design Decisions
//!
//! - **Probabilistic Sampling**: Uses probability-based sampling for unbiased
//!   random selection
//! - **Per-Item Decision**: Each item is independently sampled based on the
//!   probability
//! - **Random Number Generation**: Uses thread-local random number generator
//!   for efficiency
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Sample`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::SampleTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that randomly samples items from a stream based on probability.
///
/// This node wraps `SampleTransformer` for use in graphs. It passes through
/// each item with a configurable probability, creating a random sample of the
/// input stream. Each item is independently sampled, ensuring unbiased random
/// selection.
///
/// # Probability
///
/// The probability value determines the likelihood that each item will pass
/// through. It must be between 0.0 and 1.0 (inclusive):
///
/// - `0.0`: No items pass through
/// - `0.1`: Approximately 10% of items pass through
/// - `0.5`: Approximately 50% of items pass through
/// - `1.0`: All items pass through
///
/// # Example
///
/// ```rust
/// use streamweave::graph::nodes::{Sample, TransformerNode};
///
/// // Create a sample node that passes 10% of items
/// let sample = Sample::<i32>::new(0.1);
///
/// // Use in a graph
/// let node = TransformerNode::from_transformer(
///     "sample".to_string(),
///     sample,
/// );
/// ```
///
/// ## Sampling Behavior
///
/// ```text
/// // Input: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
/// // With probability 0.3, output might be: [2, 5, 9]
/// // (Each item independently has a 30% chance of passing through)
/// ```
pub struct Sample<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying sample transformer
  transformer: SampleTransformer<T>,
}

impl<T> Sample<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Sample` node with the specified probability.
  ///
  /// # Arguments
  ///
  /// * `probability` - The probability (0.0 to 1.0) that an item will be passed through.
  ///
  /// # Panics
  ///
  /// Panics if `probability` is not between 0.0 and 1.0 (inclusive).
  pub fn new(probability: f64) -> Self {
    Self {
      transformer: SampleTransformer::new(probability),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.transformer = self.transformer.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this node.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this node.
  pub fn with_name(mut self, name: String) -> Self {
    self.transformer = self.transformer.with_name(name);
    self
  }
}

impl<T> Clone for Sample<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: SampleTransformer::new(self.transformer.probability),
    }
    .with_error_strategy(self.transformer.config.error_strategy.clone())
    .with_name(
      self
        .transformer
        .config
        .name
        .clone()
        .unwrap_or_else(|| "sample".to_string()),
    )
  }
}

impl<T> Input for Sample<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Sample<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Sample<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
