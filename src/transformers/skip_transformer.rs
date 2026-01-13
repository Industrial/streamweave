//! Skip transformer for skipping items from the beginning of streams.
//!
//! This module provides [`SkipTransformer<T>`], a transformer that skips a specified
//! number of items from the beginning of a stream and then passes through all subsequent
//! items. It's useful for pagination, offset-based processing, and skipping headers or
//! initial data that should be ignored.
//!
//! # Overview
//!
//! [`SkipTransformer`] discards the first `skip` items from a stream and then passes
//! through all remaining items. This is the inverse of `LimitTransformer`, which limits
//! items at the beginning. Together, they enable offset and limit operations for
//! pagination and selective processing.
//!
//! # Key Concepts
//!
//! - **Skip Count**: Specifies how many items to skip from the beginning
//! - **Offset Operation**: Useful for pagination (skip first N items)
//! - **Zero-Copy Support**: Supports zero-copy transformations via `ZeroCopyTransformer`
//! - **Generic Type**: Works with any item type
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`SkipTransformer<T>`]**: Transformer that skips items from the beginning
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::SkipTransformer;
//! use streamweave::PipelineBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a transformer that skips the first 10 items
//! let transformer = SkipTransformer::new(10);
//!
//! // Input: [1, 2, 3, ..., 100]
//! // Output: [11, 12, 13, ..., 100]  (first 10 items skipped)
//! # Ok(())
//! # }
//! ```
//!
//! ## Pagination Pattern
//!
//! ```rust
//! use streamweave::transformers::SkipTransformer;
//!
//! // Skip 50 items for page 2 (assuming 50 items per page)
//! let transformer = SkipTransformer::new(50)
//!     .with_name("page-2-skip".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Skip Operation**: Uses `StreamExt::skip` for efficient stream skipping
//! - **Zero-Copy Support**: Implements `ZeroCopyTransformer` for efficient transformations
//! - **Simple API**: Takes a skip count for straightforward configuration
//! - **Generic Type**: Generic over item type for maximum flexibility
//! - **Inverse of Limit**: Complements `LimitTransformer` for offset/limit patterns
//!
//! # Integration with StreamWeave
//!
//! [`SkipTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`]. It also implements
//! `ZeroCopyTransformer` for efficient zero-copy transformations.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
// use crate::graph::ZeroCopyTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::marker::PhantomData;
use std::pin::Pin;

/// A transformer that skips a specified number of items from the beginning of a stream.
///
/// This transformer discards the first `skip` items and then passes through
/// all subsequent items.
#[derive(Clone)]
pub struct SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The number of items to skip from the beginning of the stream.
  pub skip: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `SkipTransformer` with the given skip count.
  ///
  /// # Arguments
  ///
  /// * `skip` - The number of items to skip from the beginning of the stream.
  pub fn new(skip: usize) -> Self {
    Self {
      skip,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }
}

impl<T> Input for SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for SkipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let skip = self.skip;
    Box::pin(input.skip(skip))
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
        .unwrap_or_else(|| "skip_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "skip_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

// impl<T> ZeroCopyTransformer for SkipTransformer<T>
// where
//   T: std::fmt::Debug + Clone + Send + Sync + 'static,
// {
//   /// Transforms an item using zero-copy semantics with `Cow`.
//   ///
//   /// For `SkipTransformer`, this method handles both `Cow::Borrowed` and `Cow::Owned`
//   /// inputs. Since skipping doesn't modify items, we can return the same Cow variant
//   /// when an item should pass through (after the skip count), achieving true zero-copy.
//   ///
//   /// # Zero-Copy Behavior
//   ///
//   /// - `Cow::Borrowed`: Returns `Cow::Borrowed` when item should pass (zero-copy)
//   /// - `Cow::Owned`: Returns `Cow::Owned` when item should pass (no additional clone)
//   ///
//   /// Both cases preserve the original Cow variant when the item should pass through,
//   /// enabling zero-copy skip operations.
//   ///
//   /// # Note
//   ///
//   /// This method always returns the input Cow variant. The actual skipping logic
//   /// (discarding the first N items) is handled at the stream level in the `transform`
//   /// method. This zero-copy method is used for per-item transformations when the item
//   /// is known to pass through (after skip count).
//   fn transform_zero_copy<'a>(&mut self, input: Cow<'a, T>) -> Cow<'a, T> {
//     // Return the same Cow variant (zero-copy pass-through)
//     // The actual skipping happens at stream level in transform method
//     input
//   }
// }
