//! Filter transformer for StreamWeave

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::borrow::Cow;
use std::pin::Pin;
use streamweave::{Input, Output, Transformer, TransformerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_graph::ZeroCopyTransformer;

/// A transformer that filters stream items based on a predicate function.
///
/// Only items for which the predicate returns `true` are passed through.
pub struct FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The predicate function used to filter items.
  pub predicate: F,
  /// Phantom data to track the item type.
  pub _phantom: std::marker::PhantomData<T>,
  /// Configuration for the transformer.
  pub config: TransformerConfig<T>,
}

impl<F, T> FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `FilterTransformer` with the given predicate function.
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

impl<F, T> Input for FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<F, T> Output for FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<F, T> Transformer for FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let predicate = self.predicate.clone();
    Box::pin(input.filter(move |item| {
      let mut predicate = predicate.clone();
      futures::future::ready(predicate(item))
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
        .unwrap_or_else(|| "filter_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

impl<F, T> ZeroCopyTransformer for FilterTransformer<F, T>
where
  F: FnMut(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Transforms an item using zero-copy semantics with `Cow`.
  ///
  /// For `FilterTransformer`, this method handles both `Cow::Borrowed` and `Cow::Owned`
  /// inputs. Since filtering doesn't modify items, we can return the same Cow variant
  /// when an item passes the filter, achieving true zero-copy for filtered items.
  ///
  /// # Zero-Copy Behavior
  ///
  /// - `Cow::Borrowed`: If item passes filter, returns `Cow::Borrowed` (zero-copy)
  /// - `Cow::Owned`: If item passes filter, returns `Cow::Owned` (no additional clone)
  ///
  /// Both cases preserve the original Cow variant when the item passes the filter,
  /// enabling zero-copy filtering operations.
  ///
  /// # Note
  ///
  /// This method always returns the input Cow variant if the item passes the filter.
  /// The actual filtering logic (discarding items that don't pass) is handled at the
  /// stream level in the `transform` method. This zero-copy method is used for
  /// per-item transformations when the item is known to pass the filter.
  fn transform_zero_copy<'a>(&mut self, input: Cow<'a, T>) -> Cow<'a, T> {
    // Check if the item passes the filter
    let passes = match &input {
      Cow::Borrowed(borrowed) => {
        let mut predicate = self.predicate.clone();
        predicate(borrowed)
      }
      Cow::Owned(ref owned) => {
        let mut predicate = self.predicate.clone();
        predicate(owned)
      }
    };

    if passes {
      // Item passes filter - return the same Cow variant (zero-copy)
      input
    } else {
      // Item doesn't pass filter - we still need to return something
      // In practice, this case shouldn't be called for items that don't pass,
      // but we handle it by returning the input anyway (the stream-level
      // filtering will handle discarding)
      input
    }
  }
}
