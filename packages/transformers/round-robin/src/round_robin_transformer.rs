use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use streamweave::TransformerConfig;
use streamweave_error::ErrorStrategy;

/// Configuration for round-robin distribution behavior.
#[derive(Debug, Clone)]
pub struct RoundRobinConfig {
  /// Number of consumers to distribute to.
  pub num_consumers: usize,
  /// Whether to include consumer index with each element.
  pub include_index: bool,
}

impl Default for RoundRobinConfig {
  fn default() -> Self {
    Self {
      num_consumers: 2,
      include_index: false,
    }
  }
}

impl RoundRobinConfig {
  /// Creates a new RoundRobinConfig with the specified number of consumers.
  #[must_use]
  pub fn new(num_consumers: usize) -> Self {
    Self {
      num_consumers,
      ..Default::default()
    }
  }

  /// Sets whether to include consumer index with each element.
  #[must_use]
  pub fn with_include_index(mut self, include: bool) -> Self {
    self.include_index = include;
    self
  }
}

/// A transformer that distributes elements to consumers in round-robin fashion.
///
/// This implements load balancing where each element is sent to exactly one
/// consumer, cycling through consumers in order. This is useful for:
/// - Distributing work across multiple workers
/// - Load balancing parallel processing
/// - Partitioning data into equal-sized batches
///
/// Unlike broadcast, round-robin does NOT clone elements - each element
/// goes to exactly one consumer.
///
/// # Example
///
/// ```ignore
/// use streamweave::transformers::round_robin::round_robin_transformer::RoundRobinTransformer;
///
/// let distributor = RoundRobinTransformer::<i32>::new(3);
/// // Elements will go to consumer 0, 1, 2, 0, 1, 2, ...
/// ```
#[derive(Clone)]
pub struct RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Configuration for the transformer.
  pub config: TransformerConfig<T>,
  /// Round-robin specific configuration.
  pub rr_config: RoundRobinConfig,
  /// Current consumer index (atomic for thread safety).
  pub current_index: Arc<AtomicUsize>,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new RoundRobinTransformer with the specified number of consumers.
  #[must_use]
  pub fn new(num_consumers: usize) -> Self {
    Self {
      config: TransformerConfig::default(),
      rr_config: RoundRobinConfig::new(num_consumers),
      current_index: Arc::new(AtomicUsize::new(0)),
      _phantom: PhantomData,
    }
  }

  /// Creates a new RoundRobinTransformer with custom configuration.
  #[must_use]
  pub fn with_config(rr_config: RoundRobinConfig) -> Self {
    Self {
      config: TransformerConfig::default(),
      rr_config,
      current_index: Arc::new(AtomicUsize::new(0)),
      _phantom: PhantomData,
    }
  }

  /// Sets the error strategy for the transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the number of consumers.
  #[must_use]
  pub fn num_consumers(&self) -> usize {
    self.rr_config.num_consumers
  }

  /// Gets the next consumer index and advances the counter.
  pub fn next_consumer(&self) -> usize {
    let num = self.rr_config.num_consumers;
    if num == 0 {
      return 0;
    }
    self.current_index.fetch_add(1, Ordering::SeqCst) % num
  }

  /// Resets the counter to start from consumer 0.
  pub fn reset(&self) {
    self.current_index.store(0, Ordering::SeqCst);
  }

  /// Returns the current index without advancing.
  #[must_use]
  pub fn current_consumer(&self) -> usize {
    let num = self.rr_config.num_consumers;
    if num == 0 {
      return 0;
    }
    self.current_index.load(Ordering::SeqCst) % num
  }
}

impl<T> Default for RoundRobinTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new(2)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_round_robin_config_default() {
    let config = RoundRobinConfig::default();
    assert_eq!(config.num_consumers, 2);
    assert!(!config.include_index);
  }

  #[test]
  fn test_round_robin_config_builder() {
    let config = RoundRobinConfig::new(5).with_include_index(true);

    assert_eq!(config.num_consumers, 5);
    assert!(config.include_index);
  }

  #[test]
  fn test_round_robin_transformer_new() {
    let transformer = RoundRobinTransformer::<i32>::new(3);
    assert_eq!(transformer.num_consumers(), 3);
    assert_eq!(transformer.current_consumer(), 0);
  }

  #[test]
  fn test_round_robin_transformer_next_consumer() {
    let transformer = RoundRobinTransformer::<i32>::new(3);

    assert_eq!(transformer.next_consumer(), 0);
    assert_eq!(transformer.next_consumer(), 1);
    assert_eq!(transformer.next_consumer(), 2);
    assert_eq!(transformer.next_consumer(), 0); // Wraps around
    assert_eq!(transformer.next_consumer(), 1);
  }

  #[test]
  fn test_round_robin_transformer_reset() {
    let transformer = RoundRobinTransformer::<i32>::new(3);

    transformer.next_consumer();
    transformer.next_consumer();
    assert_eq!(transformer.current_consumer(), 2);

    transformer.reset();
    assert_eq!(transformer.current_consumer(), 0);
  }

  #[test]
  fn test_round_robin_transformer_default() {
    let transformer = RoundRobinTransformer::<i32>::default();
    assert_eq!(transformer.num_consumers(), 2);
  }

  #[test]
  fn test_round_robin_transformer_clone() {
    let transformer = RoundRobinTransformer::<i32>::new(3).with_name("original".to_string());

    // Advance the counter
    transformer.next_consumer();
    transformer.next_consumer();

    let cloned = transformer.clone();
    assert_eq!(cloned.num_consumers(), 3);
    assert_eq!(cloned.config.name, Some("original".to_string()));
    // Clone shares the same Arc, so counter is shared
    assert_eq!(cloned.current_consumer(), 2);
  }

  #[test]
  fn test_round_robin_with_zero_consumers() {
    let transformer = RoundRobinTransformer::<i32>::new(0);
    // Should handle gracefully by returning 0
    assert_eq!(transformer.next_consumer(), 0);
    assert_eq!(transformer.current_consumer(), 0);
  }

  #[test]
  fn test_round_robin_with_single_consumer() {
    let transformer = RoundRobinTransformer::<i32>::new(1);

    assert_eq!(transformer.next_consumer(), 0);
    assert_eq!(transformer.next_consumer(), 0);
    assert_eq!(transformer.next_consumer(), 0);
  }
}
