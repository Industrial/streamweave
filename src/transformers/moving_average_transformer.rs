//! Moving average transformer for StreamWeave
//!
//! This transformer requires the `stateful` feature to be enabled.

#![cfg(feature = "stateful")]

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::stateful_transformer::{
  InMemoryStateStore, StateStore, StateStoreExt, StatefulTransformer,
};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::StreamExt;
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::Stream;

/// State for the moving average calculation.
///
/// Maintains a sliding window of recent values.
#[derive(Debug, Clone)]
pub struct MovingAverageState {
  /// The sliding window of values.
  pub window: VecDeque<f64>,
  /// Maximum window size.
  pub window_size: usize,
}

impl MovingAverageState {
  /// Creates a new state with the specified window size.
  pub fn new(window_size: usize) -> Self {
    Self {
      window: VecDeque::with_capacity(window_size),
      window_size,
    }
  }

  /// Adds a value to the window, removing the oldest if at capacity.
  pub fn add_value(&mut self, value: f64) {
    if self.window.len() >= self.window_size {
      self.window.pop_front();
    }
    self.window.push_back(value);
  }

  /// Calculates the current average.
  pub fn average(&self) -> f64 {
    if self.window.is_empty() {
      return 0.0;
    }
    let sum: f64 = self.window.iter().sum();
    sum / self.window.len() as f64
  }
}

/// A stateful transformer that calculates a moving average over a sliding window.
///
/// # Example
///
/// ```rust
/// use crate::transformers::MovingAverageTransformer;
/// use crate::transformer::Transformer;
/// use futures::StreamExt;
///
/// # async fn example() {
/// let mut transformer = MovingAverageTransformer::new(3); // 3-item window
/// let input = Box::pin(futures::stream::iter(vec![1.0, 2.0, 3.0, 4.0, 5.0]));
/// let output = transformer.transform(input).await;
/// let results: Vec<f64> = output.collect().await;
/// // Window: [1] -> avg 1.0
/// // Window: [1,2] -> avg 1.5
/// // Window: [1,2,3] -> avg 2.0
/// // Window: [2,3,4] -> avg 3.0
/// // Window: [3,4,5] -> avg 4.0
/// assert_eq!(results, vec![1.0, 1.5, 2.0, 3.0, 4.0]);
/// # }
/// ```
#[derive(Debug)]
pub struct MovingAverageTransformer {
  /// Configuration for the transformer.
  pub(crate) config: TransformerConfig<f64>,
  /// State store for maintaining the window (wrapped in Arc for sharing).
  pub(crate) state_store: Arc<InMemoryStateStore<MovingAverageState>>,
  /// Window size for the moving average.
  pub(crate) window_size: usize,
}

impl Clone for MovingAverageTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      state_store: Arc::clone(&self.state_store),
      window_size: self.window_size,
    }
  }
}

impl MovingAverageTransformer {
  /// Creates a new MovingAverageTransformer with the specified window size.
  ///
  /// # Arguments
  ///
  /// * `window_size` - The number of recent items to include in the average.
  ///
  /// # Panics
  ///
  /// Panics if window_size is 0.
  pub fn new(window_size: usize) -> Self {
    assert!(window_size > 0, "Window size must be greater than 0");
    Self {
      config: TransformerConfig::default(),
      state_store: Arc::new(InMemoryStateStore::new(MovingAverageState::new(
        window_size,
      ))),
      window_size,
    }
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Sets the error strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<f64>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Returns the window size.
  pub fn window_size(&self) -> usize {
    self.window_size
  }
}

impl Input for MovingAverageTransformer {
  type Input = f64;
  type InputStream = Pin<Box<dyn Stream<Item = f64> + Send>>;
}

impl Output for MovingAverageTransformer {
  type Output = f64;
  type OutputStream = Pin<Box<dyn Stream<Item = f64> + Send>>;
}

/// Wrapper struct to implement StateStore on `Arc<InMemoryStateStore<MovingAverageState>>`
#[derive(Debug)]
pub struct SharedMovingAverageStore(pub Arc<InMemoryStateStore<MovingAverageState>>);

impl Clone for SharedMovingAverageStore {
  fn clone(&self) -> Self {
    Self(Arc::clone(&self.0))
  }
}

impl StateStore<MovingAverageState> for SharedMovingAverageStore {
  fn get(&self) -> crate::stateful_transformer::StateResult<Option<MovingAverageState>> {
    self.0.get()
  }

  fn set(&self, state: MovingAverageState) -> crate::stateful_transformer::StateResult<()> {
    self.0.set(state)
  }

  fn update_with(
    &self,
    f: Box<dyn FnOnce(Option<MovingAverageState>) -> MovingAverageState + Send>,
  ) -> crate::stateful_transformer::StateResult<MovingAverageState> {
    self.0.update_with(f)
  }

  fn reset(&self) -> crate::stateful_transformer::StateResult<()> {
    self.0.reset()
  }

  fn is_initialized(&self) -> bool {
    self.0.is_initialized()
  }

  fn initial_state(&self) -> Option<MovingAverageState> {
    self.0.initial_state()
  }
}

#[async_trait]
impl Transformer for MovingAverageTransformer {
  type InputPorts = (f64,);
  type OutputPorts = (f64,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let state_store_clone = Arc::clone(&self.state_store);
    let window_size = self.window_size;

    input
      .map(move |item| {
        state_store_clone
          .update(move |current_opt| {
            let mut state = current_opt.unwrap_or_else(|| MovingAverageState::new(window_size));
            state.add_value(item);
            state
          })
          .map(|state| state.average())
          .unwrap_or(0.0)
      })
      .boxed()
  }

  fn set_config_impl(&mut self, config: TransformerConfig<f64>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<f64> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<f64> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<f64>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<f64>) -> ErrorContext<f64> {
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
        .unwrap_or_else(|| "moving_average_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

impl StatefulTransformer for MovingAverageTransformer {
  type State = MovingAverageState;
  type Store = SharedMovingAverageStore;

  fn state_store(&self) -> &Self::Store {
    unreachable!("Use state(), set_state(), reset_state() directly")
  }

  fn state_store_mut(&mut self) -> &mut Self::Store {
    unreachable!("Use state(), set_state(), reset_state() directly")
  }

  fn state(&self) -> crate::stateful_transformer::StateResult<Option<Self::State>> {
    self.state_store.get()
  }

  fn set_state(&self, state: Self::State) -> crate::stateful_transformer::StateResult<()> {
    self.state_store.set(state)
  }

  fn reset_state(&self) -> crate::stateful_transformer::StateResult<()> {
    self.state_store.reset()
  }

  fn has_state(&self) -> bool {
    self.state_store.is_initialized()
  }

  fn update_state<F>(&self, f: F) -> crate::stateful_transformer::StateResult<Self::State>
  where
    F: FnOnce(Option<Self::State>) -> Self::State + Send + 'static,
  {
    self.state_store.update_with(Box::new(f))
  }
}
