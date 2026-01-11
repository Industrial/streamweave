//! # Zip Transformer
//!
//! Transformer for combining items from multiple input streams into vectors in StreamWeave pipelines.
//!
//! This module provides [`ZipTransformer`], a transformer that takes a stream of vectors and
//! transposes them, combining items at corresponding indices into output vectors. It enables
//! synchronizing and combining data from parallel streams by aligning items by their position.
//!
//! # Overview
//!
//! [`ZipTransformer`] is useful for combining multiple vectors of items in streaming pipelines.
//! It transposes input vectors, taking the i-th element from each input vector and combining
//! them into a new vector. This is useful for parallel processing scenarios where you need
//! to combine corresponding items from multiple sources.
//!
//! # Key Concepts
//!
//! - **Vector Transposition**: Takes streams of vectors and transposes them
//! - **Index-Based Combination**: Combines items at corresponding indices
//! - **Flexible Lengths**: Handles vectors of different lengths gracefully
//! - **Stream Buffering**: Buffers all input vectors before transposing
//! - **Synchronization**: Aligns items by position across vectors
//!
//! # Core Types
//!
//! - **[`ZipTransformer<T>`]**: Transformer that zips items from multiple input vectors
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::ZipTransformer;
//!
//! // Create a zip transformer for zipping vectors of integers
//! let transformer = ZipTransformer::<i32>::new();
//!
//! // Input stream: [vec![1, 2, 3], vec![4, 5]]
//! // Output: [vec![1, 4], vec![2, 5], vec![3]] (stops at shortest vector's length by default)
//! ```
//!
//! ## Example Transformation
//!
//! ```rust
//! use streamweave::transformers::ZipTransformer;
//!
//! let transformer = ZipTransformer::<i32>::new();
//! // Input: [vec![1, 2, 3], vec![10, 20, 30], vec![100, 200]]
//! // Output: [vec![1, 10, 100], vec![2, 20, 200]]
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::ZipTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a zip transformer with error handling strategy
//! let transformer = ZipTransformer::<String>::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("zip-vectors".to_string());
//! ```
//!
//! # Design Decisions
//!
//! ## Vector-Based Input
//!
//! Takes streams of vectors for flexible multi-item processing. This design allows
//! combining multiple vectors in a single operation, making it suitable for batch
//! processing scenarios.
//!
//! ## Transposition Operation
//!
//! Uses transpose operation to combine corresponding items. Items at the same index
//! across all input vectors are combined into a single output vector.
//!
//! ## Length Handling
//!
//! Handles vectors of different lengths by only combining available items at each
//! index. This prevents errors when vectors have mismatched lengths while still
//! producing useful output.
//!
//! ## Buffering Strategy
//!
//! Buffers all input vectors before transposing. This allows the transformer to
//! determine the maximum length and handle varying vector sizes correctly, but
//! requires memory proportional to the input size.
//!
//! ## Empty Vector Handling
//!
//! Empty vectors are skipped in the output, and empty input streams result in
//! empty output streams. This ensures consistent behavior across edge cases.
//!
//! # Integration with StreamWeave
//!
//! [`ZipTransformer`] integrates seamlessly with StreamWeave's pipeline and graph systems:
//!
//! - **Pipeline API**: Use in pipelines for vector combination operations
//! - **Graph API**: Wrap in graph nodes for graph-based vector zipping
//! - **Error Handling**: Supports standard error handling strategies
//! - **Configuration**: Supports configuration via [`TransformerConfig`]
//! - **Type Safety**: Works with any `Send + Sync + Clone` type
//!
//! # Common Patterns
//!
//! ## Combining Parallel Streams
//!
//! Combine items from parallel processing streams:
//!
//! ```rust
//! use streamweave::transformers::ZipTransformer;
//!
//! // Zip vectors from parallel processing
//! let transformer = ZipTransformer::<i32>::new();
//! ```
//!
//! ## Aligning Data by Index
//!
//! Align data from multiple sources by index:
//!
//! ```rust
//! use streamweave::transformers::ZipTransformer;
//!
//! // Align vectors from different sources
//! let transformer = ZipTransformer::<serde_json::Value>::new();
//! ```
//!
//! ## Transposing Matrix-Like Data
//!
//! Transpose matrix-like structures represented as vectors:
//!
//! ```rust
//! use streamweave::transformers::ZipTransformer;
//!
//! // Transpose rows into columns
//! let transformer = ZipTransformer::<f64>::new();
//! ```
//!
//! # Output Format
//!
//! ## Transposed Vectors
//!
//! Each output vector contains one item from each input vector at the same index:
//!
//! ```rust,no_run
//! # use streamweave::transformers::ZipTransformer;
//! // Example: Zip transformer with equal-length vectors
//! // Input: [vec![1, 2], vec![3, 4], vec![5, 6]]
//! // Output: [vec![1, 3, 5], vec![2, 4, 6]]
//! let transformer = ZipTransformer::<i32>::new();
//! let _input = vec![vec![1, 2], vec![3, 4], vec![5, 6]];
//! ```
//!
//! ## Handling Different Lengths
//!
//! When input vectors have different lengths, only items at indices that exist
//! in all vectors are included:
//!
//! ```rust,no_run
//! # use streamweave::transformers::ZipTransformer;
//! // Example: Zip transformer with different-length vectors
//! // Input: [vec![1, 2, 3], vec![4, 5]]
//! // Output: [vec![1, 4], vec![2, 5]]
//! // Note: Item 3 is skipped because second vector has no third element
//! let transformer = ZipTransformer::<i32>::new();
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::marker::PhantomData;
use std::pin::Pin;

/// A transformer that zips items from multiple streams into vectors.
///
/// This transformer collects items from multiple input streams and combines
/// them into vectors, emitting one vector per combination of items from each stream.
#[derive(Clone)]
pub struct ZipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<Vec<T>>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> Default for ZipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> ZipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `ZipTransformer`.
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Vec<T>>) -> Self {
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

impl<T> Input for ZipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

impl<T> Output for ZipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

#[async_trait]
impl<T> Transformer for ZipTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (Vec<T>,);
  type OutputPorts = (Vec<T>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(async_stream::stream! {
      let mut input = input;
      let mut buffers: Vec<Vec<T>> = Vec::new();

      while let Some(items) = input.next().await {
        buffers.push(items);
      }

      // Early return if no input
      if buffers.is_empty() {
        return;
      }

      // Get the length of the longest vector
      let max_len = buffers.iter().map(|v| v.len()).max().unwrap_or(0);

      // Yield transposed vectors
      for i in 0..max_len {
        let mut result = Vec::new();
        for buffer in &buffers {
          if let Some(item) = buffer.get(i) {
            result.push(item.clone());
          }
        }
        if !result.is_empty() {
          yield result;
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Vec<T>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Vec<T>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Vec<T>> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Vec<T>>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Vec<T>>) -> ErrorContext<Vec<T>> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "zip_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "zip_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
