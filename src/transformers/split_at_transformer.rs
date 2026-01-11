//! # Split At Transformer
//!
//! Transformer that splits a stream of items at a specific index, producing
//! two groups: items before the index and items at or after the index. This
//! enables partitioning streams based on position.
//!
//! ## Overview
//!
//! The Split At Transformer provides:
//!
//! - **Index-Based Splitting**: Splits stream at a specific index position
//! - **Two Groups**: Produces items before index and items at/after index
//! - **Full Buffering**: Collects all items before splitting (requires memory for all items)
//! - **Type Generic**: Works with any `Send + Sync + Clone` type
//! - **Error Handling**: Configurable error strategies
//!
//! ## Input/Output
//!
//! - **Input**: `Message<T>` - Items to split
//! - **Output**: `Message<Vec<T>>` - Two vectors: [items_before_index, items_at_or_after_index]
//!
//! ## Performance Considerations
//!
//! This transformer buffers all items in memory before splitting, so it's not
//! suitable for very large streams.
//!
//! ## Example
//!
//! ```rust
//! use crate::transformers::SplitAtTransformer;
//!
//! let transformer = SplitAtTransformer::new(3);
//! // Input: [1, 2, 3, 4, 5]
//! // Output: [vec![1, 2, 3], vec![4, 5]] (split at index 3)
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::marker::PhantomData;
use std::pin::Pin;

/// A transformer that splits a stream of items at a specific index.
///
/// This transformer collects items from the input stream and splits them
/// into two groups: items before the index and items at or after the index.
#[derive(Clone)]
pub struct SplitAtTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The index at which to split the stream.
  pub index: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> SplitAtTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `SplitAtTransformer` with the given index.
  ///
  /// # Arguments
  ///
  /// * `index` - The index at which to split the stream.
  pub fn new(index: usize) -> Self {
    Self {
      index,
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

impl<T> Input for SplitAtTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for SplitAtTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = (Vec<T>, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (Vec<T>, Vec<T>)> + Send>>;
}

#[async_trait]
impl<T> Transformer for SplitAtTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = ((Vec<T>, Vec<T>),);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let index = self.index;
    Box::pin(futures::stream::unfold(
      (input, index),
      |(mut input, index)| async move {
        let mut items = Vec::new();
        while let Some(item) = input.next().await {
          items.push(item);
        }
        if items.is_empty() {
          None
        } else {
          // Ensure index is within bounds to prevent panic
          let safe_index = std::cmp::min(index, items.len());
          let (first, second) = items.split_at(safe_index);
          Some((
            (first.to_vec(), second.to_vec()),
            (
              Box::pin(futures::stream::empty()) as Pin<Box<dyn Stream<Item = T> + Send>>,
              index,
            ),
          ))
        }
      },
    ))
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
        .unwrap_or_else(|| "split_at_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "split_at_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
