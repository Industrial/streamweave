use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

/// Configuration for broadcast behavior.
#[derive(Debug, Clone)]
pub struct BroadcastConfig {
  /// Number of consumers to broadcast to.
  pub num_consumers: usize,
  /// Buffer size for each consumer channel.
  /// If a consumer is slow, elements will be buffered up to this limit.
  pub buffer_size: usize,
  /// Whether to drop elements when buffer is full (instead of blocking).
  pub drop_on_overflow: bool,
}

impl Default for BroadcastConfig {
  fn default() -> Self {
    Self {
      num_consumers: 2,
      buffer_size: 100,
      drop_on_overflow: false,
    }
  }
}

impl BroadcastConfig {
  /// Creates a new BroadcastConfig with the specified number of consumers.
  #[must_use]
  pub fn new(num_consumers: usize) -> Self {
    Self {
      num_consumers,
      ..Default::default()
    }
  }

  /// Sets the buffer size for each consumer.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.buffer_size = size;
    self
  }

  /// Sets whether to drop elements when buffer is full.
  #[must_use]
  pub fn with_drop_on_overflow(mut self, drop: bool) -> Self {
    self.drop_on_overflow = drop;
    self
  }
}

/// A transformer that broadcasts each input element to multiple output streams.
///
/// This implements the fan-out pattern where each element is cloned and sent
/// to all downstream consumers. This is useful for:
/// - Duplicating a stream for parallel processing
/// - Sending data to multiple sinks (e.g., logging and processing)
/// - Creating multiple views of the same data
///
/// # Example
///
/// ```ignore
/// use streamweave::transformers::broadcast::broadcast_transformer::BroadcastTransformer;
///
/// let broadcaster = BroadcastTransformer::<i32>::new(3)
///     .with_buffer_size(50);
/// ```
#[derive(Clone)]
pub struct BroadcastTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Configuration for the transformer.
  pub config: TransformerConfig<T>,
  /// Broadcast-specific configuration.
  pub broadcast_config: BroadcastConfig,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> BroadcastTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new BroadcastTransformer with the specified number of consumers.
  #[must_use]
  pub fn new(num_consumers: usize) -> Self {
    Self {
      config: TransformerConfig::default(),
      broadcast_config: BroadcastConfig::new(num_consumers),
      _phantom: PhantomData,
    }
  }

  /// Creates a new BroadcastTransformer with custom broadcast config.
  #[must_use]
  pub fn with_config(broadcast_config: BroadcastConfig) -> Self {
    Self {
      config: TransformerConfig::default(),
      broadcast_config,
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

  /// Sets the buffer size for each consumer.
  #[must_use]
  pub fn with_buffer_size(mut self, size: usize) -> Self {
    self.broadcast_config.buffer_size = size;
    self
  }

  /// Sets whether to drop elements when buffer is full.
  #[must_use]
  pub fn with_drop_on_overflow(mut self, drop: bool) -> Self {
    self.broadcast_config.drop_on_overflow = drop;
    self
  }

  /// Returns the number of consumers.
  #[must_use]
  pub fn num_consumers(&self) -> usize {
    self.broadcast_config.num_consumers
  }

  /// Returns the buffer size.
  #[must_use]
  pub fn buffer_size(&self) -> usize {
    self.broadcast_config.buffer_size
  }
}

impl<T> Default for BroadcastTransformer<T>
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
  fn test_broadcast_config_default() {
    let config = BroadcastConfig::default();
    assert_eq!(config.num_consumers, 2);
    assert_eq!(config.buffer_size, 100);
    assert!(!config.drop_on_overflow);
  }

  #[test]
  fn test_broadcast_config_builder() {
    let config = BroadcastConfig::new(5)
      .with_buffer_size(200)
      .with_drop_on_overflow(true);

    assert_eq!(config.num_consumers, 5);
    assert_eq!(config.buffer_size, 200);
    assert!(config.drop_on_overflow);
  }

  #[test]
  fn test_broadcast_transformer_new() {
    let transformer = BroadcastTransformer::<i32>::new(3);
    assert_eq!(transformer.num_consumers(), 3);
    assert_eq!(transformer.buffer_size(), 100);
  }

  #[test]
  fn test_broadcast_transformer_with_config() {
    let config = BroadcastConfig::new(4).with_buffer_size(50);
    let transformer = BroadcastTransformer::<i32>::with_config(config);

    assert_eq!(transformer.num_consumers(), 4);
    assert_eq!(transformer.buffer_size(), 50);
  }

  #[test]
  fn test_broadcast_transformer_builder() {
    let transformer = BroadcastTransformer::<i32>::new(2)
      .with_buffer_size(25)
      .with_drop_on_overflow(true)
      .with_name("test_broadcaster".to_string());

    assert_eq!(transformer.num_consumers(), 2);
    assert_eq!(transformer.buffer_size(), 25);
    assert!(transformer.broadcast_config.drop_on_overflow);
    assert_eq!(
      transformer.config.name,
      Some("test_broadcaster".to_string())
    );
  }

  #[test]
  fn test_broadcast_transformer_default() {
    let transformer = BroadcastTransformer::<i32>::default();
    assert_eq!(transformer.num_consumers(), 2);
    assert_eq!(transformer.buffer_size(), 100);
  }

  #[test]
  fn test_broadcast_transformer_clone() {
    let transformer = BroadcastTransformer::<i32>::new(3)
      .with_buffer_size(50)
      .with_name("original".to_string());

    let cloned = transformer.clone();
    assert_eq!(cloned.num_consumers(), 3);
    assert_eq!(cloned.buffer_size(), 50);
    assert_eq!(cloned.config.name, Some("original".to_string()));
  }
}
