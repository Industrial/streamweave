//! # Zip Node
//!
//! Graph node that zips items from multiple input streams by transposing vectors
//! of items. This module provides [`Zip`], a graph node that takes streams of
//! vectors and transposes them, combining items at corresponding indices into
//! output vectors. It wraps [`ZipTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`Zip`] is useful for combining multiple vectors of items in graph-based
//! pipelines. It transposes input vectors, taking the i-th element from each
//! input vector and combining them into a new vector. This is useful for
//! parallel processing scenarios where you need to combine corresponding items
//! from multiple sources.
//!
//! # Key Concepts
//!
//! - **Vector Transposition**: Takes vectors of items and transposes them
//! - **Index-Based Combination**: Combines items at corresponding indices
//! - **Flexible Lengths**: Handles vectors of different lengths gracefully
//! - **Transformer Wrapper**: Wraps `ZipTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Zip<T>`]**: Node that zips items from multiple input vectors
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Zip;
//!
//! // Create a zip node for zipping vectors of integers
//! let zip = Zip::<i32>::new();
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Zip;
//! use streamweave::ErrorStrategy;
//!
//! // Create a zip node with error handling strategy
//! let zip = Zip::<String>::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("zip-vectors".to_string());
//! ```
//!
//! ## Example Transformation
//!
//! ```text
//! // Input stream contains vectors:
//! // vec![vec![1, 2, 3], vec![10, 20, 30]]
//!
//! // Output will be transposed:
//! // vec![vec![1, 10], vec![2, 20], vec![3, 30]]
//! ```
//!
//! # How It Works
//!
//! The zip node takes a stream of vectors, collects all vectors, then transposes
//! them by taking items at corresponding indices. For example, if you have vectors
//! `[1, 2, 3]` and `[10, 20, 30]`, the zip operation produces `[vec![1, 10], vec![2, 20], vec![3, 30]]`.
//!
//! # Design Decisions
//!
//! - **Vector-Based Input**: Takes streams of vectors for flexible multi-item processing
//! - **Transposition**: Uses transpose operation to combine corresponding items
//! - **Length Handling**: Handles vectors of different lengths by only combining
//!   available items at each index
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Zip`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::ZipTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that zips items from multiple input streams by transposing vectors.
///
/// This node wraps `ZipTransformer` for use in graphs. It takes a stream of
/// vectors and transposes them, combining items at corresponding indices into
/// output vectors. This is useful for parallel processing scenarios where you
/// need to combine corresponding items from multiple sources.
///
/// # Behavior
///
/// The node collects all input vectors, then transposes them by taking the i-th
/// element from each vector and combining them into a new vector. Vectors of
/// different lengths are handled by only including items that exist at each index.
///
/// # Input/Output
///
/// - **Input**: Stream of `Vec<T>` - Vectors of items to zip
/// - **Output**: Stream of `Vec<T>` - Transposed vectors combining items at
///   corresponding indices
///
/// # Example
///
/// ```rust
/// use streamweave::graph::nodes::{Zip, TransformerNode};
///
/// // Create a zip node for zipping vectors of integers
/// let zip = Zip::<i32>::new();
///
/// // Use in a graph
/// let node = TransformerNode::from_transformer(
///     "zip".to_string(),
///     zip,
/// );
/// ```
///
/// ## Transformation Example
///
/// ```text
/// // If input stream contains:
/// // vec![vec![1, 2, 3], vec![10, 20, 30]]
///
/// // Output will be:
/// // vec![vec![1, 10], vec![2, 20], vec![3, 30]]
/// ```
pub struct Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying zip transformer
  transformer: ZipTransformer<T>,
}

impl<T> Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Zip` node.
  pub fn new() -> Self {
    Self {
      transformer: ZipTransformer::new(),
    }
  }

  /// Sets the error handling strategy for this node.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Vec<T>>) -> Self {
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

impl<T> Default for Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = Vec<T>;
  type InputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

impl<T> Output for Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Vec<T>;
  type OutputStream = Pin<Box<dyn Stream<Item = Vec<T>> + Send>>;
}

#[async_trait]
impl<T> Transformer for Zip<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (Vec<T>,);
  type OutputPorts = (Vec<T>,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    self.transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Vec<T>>) {
    self.transformer.set_config_impl(config);
  }

  fn get_config_impl(&self) -> &TransformerConfig<Vec<T>> {
    self.transformer.get_config_impl()
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Vec<T>> {
    self.transformer.get_config_mut_impl()
  }

  fn handle_error(&self, error: &StreamError<Vec<T>>) -> ErrorAction {
    self.transformer.handle_error(error)
  }

  fn create_error_context(&self, item: Option<Vec<T>>) -> ErrorContext<Vec<T>> {
    self.transformer.create_error_context(item)
  }

  fn component_info(&self) -> ComponentInfo {
    self.transformer.component_info()
  }
}
