//! Take transformer for StreamWeave

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::graph::ZeroCopyTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::borrow::Cow;
use std::marker::PhantomData;
use std::pin::Pin;

/// A transformer that takes a specified number of items from a stream.
///
/// This transformer passes through only the first `take` items from the input
/// stream and then stops producing items.
#[derive(Clone)]
pub struct TakeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The number of items to take from the stream.
  pub take: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> TakeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `TakeTransformer` with the given take count.
  ///
  /// # Arguments
  ///
  /// * `take` - The number of items to take from the stream.
  pub fn new(take: usize) -> Self {
    Self {
      take,
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

impl<T> Input for TakeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for TakeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for TakeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let take = self.take;
    Box::pin(input.take(take))
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
        .unwrap_or_else(|| "take_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "take_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

impl<T> ZeroCopyTransformer for TakeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Transforms an item using zero-copy semantics with `Cow`.
  ///
  /// For `TakeTransformer`, this method handles both `Cow::Borrowed` and `Cow::Owned`
  /// inputs. Since taking doesn't modify items, we can return the same Cow variant
  /// when an item should pass through (within the take count), achieving true zero-copy.
  ///
  /// # Zero-Copy Behavior
  ///
  /// - `Cow::Borrowed`: Returns `Cow::Borrowed` when item should pass (zero-copy)
  /// - `Cow::Owned`: Returns `Cow::Owned` when item should pass (no additional clone)
  ///
  /// Both cases preserve the original Cow variant when the item should pass through,
  /// enabling zero-copy take operations.
  ///
  /// # Note
  ///
  /// This method always returns the input Cow variant. The actual taking logic
  /// (limiting to first N items) is handled at the stream level in the `transform`
  /// method. This zero-copy method is used for per-item transformations when the item
  /// is known to pass through (within take count).
  fn transform_zero_copy<'a>(&mut self, input: Cow<'a, T>) -> Cow<'a, T> {
    // Return the same Cow variant (zero-copy pass-through)
    // The actual taking happens at stream level in transform method
    input
  }
}
