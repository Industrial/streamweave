//! Merge transformer for combining multiple streams into one.
//!
//! This module provides [`MergeTransformer<T>`], a transformer that merges multiple
//! input streams into a single output stream. Items are interleaved as they become
//! available, making it useful for combining data from multiple sources, parallel
//! processing results, or aggregating streams from different producers.
//!
//! # Overview
//!
//! [`MergeTransformer`] combines items from multiple streams into a single stream.
//! It uses fair interleaving (via `futures::stream::select_all`), meaning items are
//! emitted as soon as they're available from any stream. This provides efficient
//! merging without blocking on slower streams.
//!
//! # Key Concepts
//!
//! - **Stream Merging**: Combines multiple streams into one
//! - **Fair Interleaving**: Items are interleaved as they become available
//! - **Dynamic Streams**: Streams can be added using `add_stream`
//! - **Non-Blocking**: Doesn't block on slower streams
//! - **Generic Type**: Works with any item type
//!
//! # Core Types
//!
//! - **[`MergeTransformer<T>`]**: Transformer that merges multiple streams
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::MergeTransformer;
//! use streamweave::PipelineBuilder;
//! use futures::stream;
//! use std::pin::Pin;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a merge transformer
//! let mut merger = MergeTransformer::<i32>::new();
//!
//! // Add streams to merge
//! merger.add_stream(Box::pin(stream::iter(vec![1, 2, 3])));
//! merger.add_stream(Box::pin(stream::iter(vec![4, 5, 6])));
//!
//! // Output: Interleaved items from both streams (order depends on availability)
//! # Ok(())
//! # }
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::MergeTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a merger with error handling
//! let merger = MergeTransformer::<String>::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("stream-merger".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Fair Interleaving**: Uses `select_all` for fair, non-blocking interleaving
//! - **Dynamic Streams**: Supports adding streams dynamically via `add_stream`
//! - **Stream Consumption**: Takes ownership of streams for efficient processing
//! - **Generic Type**: Generic over item type for maximum flexibility
//! - **Simple Merging**: Straightforward merge operation without complex ordering
//!
//! # Integration with StreamWeave
//!
//! [`MergeTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::marker::PhantomData;
use std::pin::Pin;

/// A transformer that merges multiple streams into a single stream.
///
/// This transformer combines items from multiple input streams into a single output stream,
/// interleaving items as they become available.
pub struct MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// The streams to merge together.
  pub streams: Vec<Pin<Box<dyn Stream<Item = T> + Send>>>,
}

impl<T> Clone for MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      _phantom: self._phantom,
      config: self.config.clone(),
      streams: Vec::new(), // Streams can't be cloned, so start with empty
    }
  }
}

impl<T> MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `MergeTransformer` with no streams.
  ///
  /// Use `add_stream` to add streams to merge.
  pub fn new() -> Self {
    Self {
      _phantom: PhantomData,
      config: TransformerConfig::default(),
      streams: Vec::new(),
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

  /// Adds a stream to be merged.
  ///
  /// # Arguments
  ///
  /// * `stream` - The stream to add to the merge operation.
  pub fn add_stream(&mut self, stream: Pin<Box<dyn Stream<Item = T> + Send>>) {
    self.streams.push(stream);
  }
}

impl<T> Default for MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Input for MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for MergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let mut all_streams = vec![input];
    all_streams.extend(std::mem::take(&mut self.streams));
    Box::pin(futures::stream::select_all(all_streams))
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
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "merge_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
