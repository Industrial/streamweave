//! Drop transformer for StreamWeave
//!
//! Drops items from the stream based on a predicate function. This is the inverse
//! of FilterTransformer - items where the predicate returns `true` are dropped.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// A transformer that drops items from the stream based on a predicate function.
///
/// This transformer is the inverse of `FilterTransformer`. Items for which the
/// predicate returns `true` are dropped (not passed through), while items where
/// the predicate returns `false` are kept.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::DropTransformer;
///
/// // Drop all items greater than 10
/// let transformer = DropTransformer::new(|x: &i32| *x > 10);
/// // Input: [5, 15, 8, 20, 3]
/// // Output: [5, 8, 3]
/// ```
pub struct DropTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The predicate function used to determine which items to drop.
  pub predicate: F,
  /// Phantom data to track the item type.
  pub _phantom: std::marker::PhantomData<T>,
  /// Configuration for the transformer.
  pub config: TransformerConfig<T>,
}

impl<F, T> DropTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `DropTransformer` with the given predicate function.
  ///
  /// Items for which the predicate returns `true` will be dropped.
  ///
  /// # Arguments
  ///
  /// * `predicate` - The function to use for determining which items to drop.
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      _phantom: std::marker::PhantomData,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<F, T> Clone for DropTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      predicate: self.predicate.clone(),
      _phantom: std::marker::PhantomData,
      config: self.config.clone(),
    }
  }
}

impl<F, T> Input for DropTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<F, T> Output for DropTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<F, T> Transformer for DropTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let predicate = self.predicate.clone();
    // Drop items where predicate returns true (inverse of filter)
    Box::pin(input.filter(move |item| {
      let mut predicate = predicate.clone();
      futures::future::ready(!predicate(item))
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
        .unwrap_or_else(|| "drop_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
