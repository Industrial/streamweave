//! Interleave node for interleaving items from multiple streams.
//!
//! This module provides [`Interleave`], a graph node that interleaves items from
//! two streams. It alternates between items from the input stream and items from
//! another stream, creating an interleaved output stream. It wraps
//! [`InterleaveTransformer`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`Interleave`] is useful for combining multiple streams by alternating between
//! them in graph-based pipelines. It takes items from two streams and interleaves
//! them, creating a single output stream that alternates between the two sources.
//!
//! # Key Concepts
//!
//! - **Stream Interleaving**: Alternates items from two input streams
//! - **Dual Input**: Takes items from the main input stream and another stream
//! - **Alternating Output**: Produces an output stream that alternates between
//!   items from both sources
//! - **Transformer Wrapper**: Wraps `InterleaveTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Interleave<T>`]**: Node that interleaves items from multiple streams
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Interleave;
//! use futures::stream;
//!
//! // Create an interleave node with another stream
//! let other_stream = Box::pin(stream::empty::<i32>());
//! let interleave = Interleave::new(other_stream);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Interleave;
//! use streamweave::ErrorStrategy;
//! use futures::stream;
//!
//! # let other_stream = Box::pin(stream::empty::<i32>());
//! // Create an interleave node with error handling
//! let interleave = Interleave::new(other_stream)
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("interleaver".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Alternating Pattern**: Alternates between two streams for interleaving
//! - **Stream-Based**: Works with async streams for efficient processing
//! - **Type-Safe**: Supports generic types for flexible item types
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Interleave`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::InterleaveTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Node that interleaves items from multiple input streams.
///
/// This node wraps `InterleaveTransformer` for use in graphs. It alternates
/// between items from the input stream and items from another stream, creating
/// an interleaved output stream.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Interleave, TransformerNode};
/// use futures::stream;
///
/// let other_stream = Box::pin(stream::empty());
/// let interleave = Interleave::new(other_stream);
/// let node = TransformerNode::from_transformer(
///     "interleave".to_string(),
///     interleave,
/// );
/// ```
pub struct Interleave<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying interleave transformer
  transformer: InterleaveTransformer<T>,
}

impl<T> Interleave<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Interleave` node with the specified other stream.
  ///
  /// # Arguments
  ///
  /// * `other` - The stream to interleave with the input stream.
  pub fn new(other: Pin<Box<dyn Stream<Item = T> + Send>>) -> Self {
    Self {
      transformer: InterleaveTransformer::new(other),
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

impl<T> Clone for Interleave<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    // InterleaveTransformer can't be cloned because it contains a stream.
    // We'll create a new one with an empty stream as a placeholder.
    // In practice, the stream should be provided when creating the node.
    Self {
      transformer: InterleaveTransformer::new(Box::pin(futures::stream::empty())),
    }
    .with_error_strategy(self.transformer.config.error_strategy.clone())
    .with_name(
      self
        .transformer
        .config
        .name
        .clone()
        .unwrap_or_else(|| "interleave".to_string()),
    )
  }
}

impl<T> Input for Interleave<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Interleave<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Interleave<T>
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
