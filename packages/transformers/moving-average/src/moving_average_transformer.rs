//! Builder and configuration for the MovingAverageTransformer.

use std::collections::VecDeque;
use std::sync::Arc;
use streamweave::TransformerConfig;
use streamweave_error::ErrorStrategy;
use streamweave_stateful::InMemoryStateStore;

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
/// use streamweave::transformers::moving_average::MovingAverageTransformer;
/// use streamweave::transformer::Transformer;
/// use futures::StreamExt;
///
/// # async fn example() {
/// let mut transformer = MovingAverageTransformer::new(3); // 3-item window
/// let input = Box::pin(futures::stream::iter(vec![1.0, 2.0, 3.0, 4.0, 5.0]));
/// let output = transformer.transform(input);
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_moving_average_state_new() {
    let state = MovingAverageState::new(5);
    assert_eq!(state.window_size, 5);
    assert!(state.window.is_empty());
  }

  #[test]
  fn test_moving_average_state_add_value() {
    let mut state = MovingAverageState::new(3);
    state.add_value(10.0);
    assert_eq!(state.window.len(), 1);
    assert_eq!(state.window[0], 10.0);
  }

  #[test]
  fn test_moving_average_state_add_value_overflow() {
    let mut state = MovingAverageState::new(2);
    state.add_value(1.0);
    state.add_value(2.0);
    state.add_value(3.0); // Should remove 1.0
    assert_eq!(state.window.len(), 2);
    assert_eq!(state.window[0], 2.0);
    assert_eq!(state.window[1], 3.0);
  }

  #[test]
  fn test_moving_average_state_average_empty() {
    let state = MovingAverageState::new(3);
    assert_eq!(state.average(), 0.0);
  }

  #[test]
  fn test_moving_average_state_average_single() {
    let mut state = MovingAverageState::new(3);
    state.add_value(10.0);
    assert_eq!(state.average(), 10.0);
  }

  #[test]
  fn test_moving_average_state_average_multiple() {
    let mut state = MovingAverageState::new(3);
    state.add_value(10.0);
    state.add_value(20.0);
    state.add_value(30.0);
    assert_eq!(state.average(), 20.0);
  }

  #[test]
  fn test_moving_average_state_clone() {
    let mut state1 = MovingAverageState::new(3);
    state1.add_value(5.0);
    state1.add_value(10.0);

    let state2 = state1.clone();
    assert_eq!(state1.window, state2.window);
    assert_eq!(state1.window_size, state2.window_size);
    assert_eq!(state1.average(), state2.average());
  }

  #[test]
  fn test_moving_average_transformer_new() {
    let transformer = MovingAverageTransformer::new(5);
    assert_eq!(transformer.window_size(), 5);
  }

  #[test]
  #[should_panic(expected = "Window size must be greater than 0")]
  fn test_moving_average_transformer_new_zero_panics() {
    let _ = MovingAverageTransformer::new(0);
  }

  #[test]
  fn test_moving_average_transformer_with_name() {
    let transformer = MovingAverageTransformer::new(3).with_name("test_moving_avg".to_string());
    assert_eq!(transformer.config.name, Some("test_moving_avg".to_string()));
  }

  #[test]
  fn test_moving_average_transformer_with_error_strategy() {
    let transformer =
      MovingAverageTransformer::new(3).with_error_strategy(ErrorStrategy::<f64>::Skip);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_moving_average_transformer_clone() {
    let transformer1 = MovingAverageTransformer::new(5);
    let transformer2 = transformer1.clone();

    assert_eq!(transformer1.window_size(), transformer2.window_size());
    assert_eq!(transformer1.window_size, transformer2.window_size);
  }

  #[test]
  fn test_moving_average_transformer_chaining() {
    let transformer = MovingAverageTransformer::new(7)
      .with_error_strategy(ErrorStrategy::<f64>::Retry(3))
      .with_name("chained_moving_avg".to_string());

    assert_eq!(transformer.window_size(), 7);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Retry(3)
    ));
    assert_eq!(
      transformer.config.name,
      Some("chained_moving_avg".to_string())
    );
  }
}
