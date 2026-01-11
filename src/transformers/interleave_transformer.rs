//! Interleave transformer for StreamWeave.
//!
//! This module provides [`InterleaveTransformer`], a transformer that interleaves
//! items from two streams, alternating between items from the input stream and
//! items from another stream to create a combined output stream.
//!
//! # Overview
//!
//! [`InterleaveTransformer`] is useful for combining two streams by alternating
//! between their items. This creates an interleaved output where items from both
//! streams are mixed together in an alternating pattern.
//!
//! # Key Concepts
//!
//! - **Stream Interleaving**: Alternates between items from two streams
//! - **Non-Deterministic**: Uses `futures::stream::select` for async interleaving
//! - **Stream Combination**: Combines two streams into one output stream
//! - **Generic Type**: Works with any item type (both streams must have same type)
//! - **Error Handling**: Configurable error strategies
//!
//! # Core Types
//!
//! - **[`InterleaveTransformer<T>`]**: Transformer that interleaves two streams
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::InterleaveTransformer;
//! use streamweave::PipelineBuilder;
//! use futures::stream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create another stream to interleave with
//! let other_stream = Box::pin(stream::iter(vec![10, 20, 30]));
//!
//! // Create a transformer that interleaves with the other stream
//! let transformer = InterleaveTransformer::new(other_stream);
//!
//! // Input: [1, 2, 3]
//! // Other: [10, 20, 30]
//! // Output: [1, 10, 2, 20, 3, 30] (interleaved, order may vary due to async)
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::InterleaveTransformer;
//! use streamweave::ErrorStrategy;
//! use futures::stream;
//!
//! // Create a transformer with error handling strategy
//! let other_stream = Box::pin(stream::iter(vec![1, 2, 3]));
//! let transformer = InterleaveTransformer::new(other_stream)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("interleaver".to_string());
//! ```
//!
//! # Behavior
//!
//! The transformer uses `futures::stream::select` to interleave items from both
//! streams. The exact interleaving pattern is non-deterministic and depends on
//! which stream produces items first (async behavior). When one stream completes,
//! remaining items from the other stream continue to be emitted.
//!
//! # Design Decisions
//!
//! - **Stream Select**: Uses `futures::stream::select` for async interleaving
//! - **Non-Deterministic**: Interleaving order depends on async timing
//! - **Type Matching**: Both streams must have the same item type
//! - **Stream Ownership**: Takes ownership of the other stream
//! - **Completion Handling**: Continues with remaining items when one stream completes
//!
//! # Integration with StreamWeave
//!
//! [`InterleaveTransformer`] implements the [`Transformer`] trait and can be used
//! in any StreamWeave pipeline. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::marker::PhantomData;
use std::pin::Pin;

/// A transformer that interleaves items from two streams.
///
/// This transformer alternates between items from the input stream and items
/// from another stream, creating an interleaved output stream.
pub struct InterleaveTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The other stream to interleave with the input stream.
  pub other: Pin<Box<dyn Stream<Item = T> + Send>>,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> InterleaveTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `InterleaveTransformer` with the given other stream.
  ///
  /// # Arguments
  ///
  /// * `other` - The stream to interleave with the input stream.
  pub fn new(other: Pin<Box<dyn Stream<Item = T> + Send>>) -> Self {
    Self {
      other,
      config: TransformerConfig::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Input for InterleaveTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for InterleaveTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for InterleaveTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let other = std::mem::replace(&mut self.other, Box::pin(futures::stream::empty()));
    Box::pin(futures::stream::select(input, other))
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
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "interleave_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
