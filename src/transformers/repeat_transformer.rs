//! # Repeat Transformer
//!
//! Transformer that repeats each input item a specified number of times in the
//! output stream. This enables data amplification, testing scenarios, and creating
//! retry patterns in pipelines.
//!
//! ## Overview
//!
//! The Repeat Transformer provides:
//!
//! - **Item Repetition**: Emits each input item N times consecutively
//! - **One-to-Many**: Each input item produces multiple output items
//! - **Data Amplification**: Useful for load testing and data generation
//! - **Type Generic**: Works with any `Send + Sync + Clone` type
//! - **Error Handling**: Configurable error strategies
//!
//! ## Input/Output
//!
//! - **Input**: `Message<T>` - Items to repeat
//! - **Output**: `Message<T>` - Repeated items (each input item appears N times)
//!
//! ## Example
//!
//! ```rust
//! use streamweave::transformers::RepeatTransformer;
//!
//! let transformer = RepeatTransformer::new(3);
//! // Input: [1, 2, 3]
//! // Output: [1, 1, 1, 2, 2, 2, 3, 3, 3]
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::stream;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that repeats each item N times in the output stream.
///
/// This transformer takes each input item and emits it `count` times consecutively.
/// Useful for testing, data amplification, and creating retry patterns.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::RepeatTransformer;
///
/// let transformer = RepeatTransformer::new(3);
/// // Input: [1, 2, 3]
/// // Output: [1, 1, 1, 2, 2, 2, 3, 3, 3]
/// ```
pub struct RepeatTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Number of times to repeat each item
  count: usize,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
}

impl<T> RepeatTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `RepeatTransformer` that repeats each item the specified number of times.
  ///
  /// # Arguments
  ///
  /// * `count` - The number of times to repeat each item (must be > 0)
  ///
  /// # Panics
  ///
  /// Panics if `count` is 0.
  pub fn new(count: usize) -> Self {
    assert!(count > 0, "Repeat count must be greater than 0");
    Self {
      count,
      config: TransformerConfig::default(),
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

impl<T> Clone for RepeatTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      count: self.count,
      config: self.config.clone(),
    }
  }
}

impl<T> Input for RepeatTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for RepeatTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for RepeatTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let count = self.count;
    Box::pin(input.flat_map(move |item| stream::iter((0..count).map(move |_| item.clone()))))
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
      ErrorStrategy::Custom(ref handler) => handler(error),
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
        .unwrap_or_else(|| "repeat_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
