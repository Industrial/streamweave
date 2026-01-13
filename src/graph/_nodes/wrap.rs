//! Wrap node for wrapping items with additional metadata or envelope structures.
//!
//! This module provides [`Wrap`], a graph node that wraps items with additional
//! metadata or envelope structures. While StreamWeave automatically wraps items
//! in `Message<T>`, this node allows adding custom metadata, headers, or wrapping
//! items in custom envelope structures. It wraps [`WrapTransformer`] for use in
//! StreamWeave graphs.
//!
//! # Overview
//!
//! [`Wrap`] is useful for adding metadata or envelope structures to items in
//! graph-based pipelines. It processes items and wraps them with custom metadata
//! or headers, making it ideal for adding context, tracing information, or
//! preparing items for inter-system communication.
//!
//! # Key Concepts
//!
//! - **Item Wrapping**: Wraps items with additional metadata or envelope structures
//! - **Custom Headers**: Supports custom headers for metadata
//! - **Envelope Structures**: Supports custom envelope structures
//! - **Transformer Wrapper**: Wraps `WrapTransformer` for graph usage
//!
//! # Core Types
//!
//! - **[`Wrap<T>`]**: Node that wraps items with additional metadata
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::Wrap;
//!
//! // Create a wrap node that adds no additional metadata
//! let wrap = Wrap::<i32>::new();
//! ```
//!
//! ## With Custom Headers
//!
//! ```rust
//! use streamweave::graph::nodes::Wrap;
//! use std::collections::HashMap;
//!
//! // Create a wrap node with custom headers
//! let mut headers = HashMap::new();
//! headers.insert("source".to_string(), "pipeline".to_string());
//! headers.insert("version".to_string(), "1.0".to_string());
//! let wrap = Wrap::<String>::with_headers(headers);
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::graph::nodes::Wrap;
//! use streamweave::ErrorStrategy;
//!
//! // Create a wrap node with error handling
//! let wrap = Wrap::<i32>::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("wrapper".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Metadata Support**: Supports custom headers for flexible metadata addition
//! - **Envelope Structures**: Supports custom envelope structures for inter-system
//!   communication
//! - **Transformer Wrapper**: Wraps existing transformer for consistency with
//!   other graph nodes
//!
//! # Integration with StreamWeave
//!
//! [`Wrap`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformers::WrapTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::collections::HashMap;
use std::pin::Pin;

/// Node that wraps items with additional metadata or envelope structures.
///
/// This node wraps `WrapTransformer` for use in graphs. While StreamWeave automatically
/// wraps items in `Message<T>`, this node allows adding custom metadata, headers, or
/// wrapping items in custom envelope structures for inter-system communication.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::{Wrap, TransformerNode};
/// use std::collections::HashMap;
///
/// let mut headers = HashMap::new();
/// headers.insert("source".to_string(), "pipeline".to_string());
/// let wrap = Wrap::with_headers(headers);
/// let node = TransformerNode::from_transformer(
///     "wrap".to_string(),
///     wrap,
/// );
/// ```
pub struct Wrap<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The underlying wrap transformer
  transformer: WrapTransformer<T>,
}

impl<T> Wrap<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Wrap` node that adds no additional metadata.
  ///
  /// Items will be passed through unchanged. This is useful when you want
  /// to ensure items are properly wrapped in `Message<T>`.
  pub fn new() -> Self {
    Self {
      transformer: WrapTransformer::new(),
    }
  }

  /// Creates a new `Wrap` node that adds the specified headers to messages.
  ///
  /// # Arguments
  ///
  /// * `headers` - Headers to add to each message.
  pub fn with_headers(headers: HashMap<String, String>) -> Self {
    Self {
      transformer: WrapTransformer::with_headers(headers),
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

impl<T> Default for Wrap<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for Wrap<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      transformer: self.transformer.clone(),
    }
  }
}

impl<T> Input for Wrap<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for Wrap<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for Wrap<T>
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
