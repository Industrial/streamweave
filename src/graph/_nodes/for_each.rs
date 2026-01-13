//! ForEach node for expanding collections into individual items.
//!
//! This module provides [`ForEach`], a graph node that expands collections
//! (e.g., `Vec<T>`) into individual items. It takes items that are collections
//! and expands them into a stream of individual items, implementing a for-each
//! loop pattern. It uses zero-copy semantics where possible.
//!
//! # Overview
//!
//! [`ForEach`] is useful for processing collections in graph-based pipelines.
//! It allows each item in a collection to be processed individually, enabling
//! batch processing patterns and nested data structure handling.
//!
//! # Key Concepts
//!
//! - **Collection Expansion**: Expands collections into individual items
//! - **For-Each Pattern**: Implements a for-each loop over collection items
//! - **Extract Function**: Uses a function to extract collections from items
//! - **Zero-Copy Semantics**: Uses zero-copy semantics where possible
//!
//! # Core Types
//!
//! - **[`ForEach<T>`]**: Node that expands collections into individual items
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::ForEach;
//!
//! // Expand vectors of integers into individual integers
//! let for_each = ForEach::new(|item: &Vec<i32>| item.clone());
//! ```
//!
//! ## With Nested Collections
//!
//! ```rust
//! use streamweave::graph::nodes::ForEach;
//!
//! // Extract and expand a nested collection
//! let for_each = ForEach::new(|item: &MyStruct| item.items.clone());
//! ```
//!
//! # Design Decisions
//!
//! - **Extract Function**: Uses a function to extract collections from items
//!   for flexibility
//! - **Flat Map Pattern**: Uses flat_map internally for efficient stream
//!   expansion
//! - **Zero-Copy**: Uses zero-copy semantics where possible for performance
//! - **Collection Type**: Works with any collection that can be converted to
//!   `Vec<T>`
//!
//! # Integration with StreamWeave
//!
//! [`ForEach`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;

/// Transformer that expands collections into individual items (for-each loop).
///
/// Takes items that are collections (e.g., `Vec<T>`) and expands them into
/// a stream of individual items. Uses zero-copy semantics where possible.
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::ForEach;
/// use crate::graph::node::TransformerNode;
///
/// let for_each = ForEach::new(|item: &Vec<i32>| item.clone());
/// let node = TransformerNode::from_transformer(
///     "expand".to_string(),
///     for_each,
/// );
/// ```
pub struct ForEach<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// Function to extract collection from item
  #[allow(clippy::type_complexity)]
  extract_collection: Arc<dyn Fn(&T) -> Vec<T> + Send + Sync>,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
}

impl<T> ForEach<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `ForEach` transformer.
  ///
  /// # Arguments
  ///
  /// * `extract_collection` - Function that extracts a collection from an item.
  ///   The collection will be expanded into individual items in the output stream.
  ///
  /// # Returns
  ///
  /// A new `ForEach` transformer instance.
  pub fn new<F>(extract_collection: F) -> Self
  where
    F: Fn(&T) -> Vec<T> + Send + Sync + 'static,
  {
    Self {
      extract_collection: Arc::new(extract_collection),
      config: TransformerConfig::default(),
    }
  }
}

impl<T> Input for ForEach<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for ForEach<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for ForEach<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let extract = Arc::clone(&self.extract_collection);
    Box::pin(input.flat_map(move |item| {
      let collection = extract(&item);
      futures::stream::iter(collection)
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy() {
      crate::error::ErrorStrategy::Stop => ErrorAction::Stop,
      crate::error::ErrorStrategy::Skip => ErrorAction::Skip,
      crate::error::ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      crate::error::ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
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
        .name()
        .clone()
        .unwrap_or_else(|| "for_each".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
