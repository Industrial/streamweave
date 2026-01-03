//! Running sum transformer for StreamWeave
//!
//! This transformer requires the `stateful` feature to be enabled.

#![cfg(feature = "stateful")]

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::fmt::Debug;
use std::ops::Add;
use std::pin::Pin;
use std::sync::Arc;
use streamweave_stateful::{InMemoryStateStore, StateStore, StateStoreExt, StatefulTransformer};

/// A stateful transformer that maintains a running sum of all processed items.
///
/// # Type Parameters
///
/// * `T` - The numeric type to sum. Must implement `Add`, `Default`, `Clone`, etc.
pub struct RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  /// Configuration for the transformer.
  pub(crate) config: TransformerConfig<T>,
  /// State store for maintaining the running sum (wrapped in Arc for sharing).
  pub(crate) state_store: Arc<InMemoryStateStore<T>>,
}

impl<T> Clone for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      state_store: Arc::clone(&self.state_store),
    }
  }
}

impl<T> RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new RunningSumTransformer with default initial value.
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
      state_store: Arc::new(InMemoryStateStore::new(T::default())),
    }
  }

  /// Creates a new RunningSumTransformer with a specified initial value.
  pub fn with_initial(initial: T) -> Self {
    Self {
      config: TransformerConfig::default(),
      state_store: Arc::new(InMemoryStateStore::new(initial)),
    }
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }

  /// Sets the error strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config = self.config.with_error_strategy(strategy);
    self
  }
}

impl<T> Default for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

/// Wrapper struct to implement StateStore on `Arc<InMemoryStateStore<T>>`
#[derive(Debug)]
pub struct SharedStateStore<T: Clone + Send + Sync>(pub Arc<InMemoryStateStore<T>>);

impl<T: Clone + Send + Sync> Clone for SharedStateStore<T> {
  fn clone(&self) -> Self {
    Self(Arc::clone(&self.0))
  }
}

impl<T: Clone + Send + Sync> StateStore<T> for SharedStateStore<T> {
  fn get(&self) -> streamweave_stateful::StateResult<Option<T>> {
    self.0.get()
  }

  fn set(&self, state: T) -> streamweave_stateful::StateResult<()> {
    self.0.set(state)
  }

  fn update_with(
    &self,
    f: Box<dyn FnOnce(Option<T>) -> T + Send>,
  ) -> streamweave_stateful::StateResult<T> {
    self.0.update_with(f)
  }

  fn reset(&self) -> streamweave_stateful::StateResult<()> {
    self.0.reset()
  }

  fn is_initialized(&self) -> bool {
    self.0.is_initialized()
  }

  fn initial_state(&self) -> Option<T> {
    self.0.initial_state()
  }
}

impl<T> Input for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let state_store_clone = Arc::clone(&self.state_store);

    input
      .map(move |item| {
        state_store_clone
          .update(move |current_opt| {
            let current = current_opt.unwrap_or_default();
            current + item
          })
          .unwrap_or_else(|_| T::default())
      })
      .boxed()
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
        .unwrap_or_else(|| "running_sum_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "running_sum_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

impl<T> StatefulTransformer for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  type State = T;
  type Store = SharedStateStore<T>;

  fn state_store(&self) -> &Self::Store {
    // This is a bit awkward, but we need to return a reference to SharedStateStore
    // For now, we'll work around this by implementing the state methods directly
    unreachable!("Use state(), set_state(), reset_state() directly")
  }

  fn state_store_mut(&mut self) -> &mut Self::Store {
    unreachable!("Use state(), set_state(), reset_state() directly")
  }

  fn state(&self) -> streamweave_stateful::StateResult<Option<Self::State>> {
    self.state_store.get()
  }

  fn set_state(&self, state: Self::State) -> streamweave_stateful::StateResult<()> {
    self.state_store.set(state)
  }

  fn reset_state(&self) -> streamweave_stateful::StateResult<()> {
    self.state_store.reset()
  }

  fn has_state(&self) -> bool {
    self.state_store.is_initialized()
  }

  fn update_state<F>(&self, f: F) -> streamweave_stateful::StateResult<Self::State>
  where
    F: FnOnce(Option<Self::State>) -> Self::State + Send + 'static,
  {
    self.state_store.update_with(Box::new(f))
  }
}
