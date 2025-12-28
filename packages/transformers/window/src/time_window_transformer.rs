use std::time::Duration;
use streamweave::TransformerConfig;
use streamweave_error::ErrorStrategy;

/// A transformer that creates time-based windows of items from a stream.
///
/// This transformer groups consecutive items into windows based on time duration,
/// producing vectors of items as windows are created based on time intervals.
#[derive(Clone)]
pub struct TimeWindowTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// The duration of each window.
  pub duration: Duration,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> TimeWindowTransformer<T> {
  /// Creates a new `TimeWindowTransformer` with the given window duration.
  ///
  /// # Arguments
  ///
  /// * `duration` - The duration of each window.
  pub fn new(duration: Duration) -> Self {
    Self {
      duration,
      config: TransformerConfig::<T>::default(),
      _phantom: std::marker::PhantomData,
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
