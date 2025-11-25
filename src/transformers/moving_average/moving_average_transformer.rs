//! Builder and configuration for the MovingAverageTransformer.

use crate::error::ErrorStrategy;
use crate::stateful_transformer::InMemoryStateStore;
use crate::transformer::TransformerConfig;
use std::collections::VecDeque;
use std::sync::Arc;

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
