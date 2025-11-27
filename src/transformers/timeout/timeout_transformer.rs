use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use tokio::time::Duration;

/// A transformer that applies a timeout to stream processing.
///
/// This transformer ensures that items are processed within a specified time limit,
/// failing if the timeout is exceeded.
#[derive(Clone)]
pub struct TimeoutTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// The maximum duration allowed for processing each item.
  pub duration: Duration,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> TimeoutTransformer<T> {
  /// Creates a new `TimeoutTransformer` with the given duration.
  ///
  /// # Arguments
  ///
  /// * `duration` - The maximum duration allowed for processing each item.
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
