//! # Sort Transformer
//!
//! Transformer that sorts items in the input stream by collecting all items,
//! sorting them, and then emitting them in sorted order. This enables ordering
//! operations on stream data, though it requires buffering all items.
//!
//! ## Overview
//!
//! The Sort Transformer provides:
//!
//! - **Stream Sorting**: Sorts all items from the input stream
//! - **Full Buffering**: Collects all items before sorting (requires memory for all items)
//! - **Ord Requirement**: Items must implement `Ord` trait for comparison
//! - **Type Generic**: Works with any `Send + Sync + Clone + Ord` type
//! - **Error Handling**: Configurable error strategies
//!
//! ## Input/Output
//!
//! - **Input**: `Message<T>` - Items to sort
//! - **Output**: `Message<T>` - Sorted items (in ascending order)
//!
//! ## Performance Considerations
//!
//! This transformer buffers all items in memory before sorting, so it's not
//! suitable for very large streams. Consider using windowing or chunking for
//! large datasets.
//!
//! ## Example
//!
//! ```rust
//! use crate::transformers::SortTransformer;
//!
//! let transformer = SortTransformer::new();
//! // Input: [3, 1, 4, 1, 5]
//! // Output: [1, 1, 3, 4, 5] (sorted)
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::marker::PhantomData;
use std::pin::Pin;

/// A transformer that sorts items in a stream.
///
/// This transformer collects all items from the input stream, sorts them,
/// and then emits them in sorted order.
#[derive(Clone)]
pub struct SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> Default for SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  /// Creates a new `SortTransformer`.
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

impl<T> Input for SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for SortTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static + Ord,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(futures::stream::unfold(input, |mut input| async move {
      let mut items = Vec::new();
      while let Some(item) = input.next().await {
        items.push(item);
      }
      if items.is_empty() {
        None
      } else {
        items.sort();
        Some((
          items[0].clone(),
          (Box::pin(futures::stream::iter(items[1..].to_vec()))
            as Pin<Box<dyn Stream<Item = T> + Send>>),
        ))
      }
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
        .unwrap_or_else(|| "sort_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "sort_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
