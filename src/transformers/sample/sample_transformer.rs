use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

/// A transformer that randomly samples items from the stream.
///
/// This transformer passes through each item with a given probability,
/// effectively creating a random sample of the input stream.
pub struct SampleTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The probability (0.0 to 1.0) that an item will be passed through.
  pub probability: f64,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> SampleTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `SampleTransformer` with the given probability.
  ///
  /// # Arguments
  ///
  /// * `probability` - The probability (0.0 to 1.0) that an item will be passed through.
  ///
  /// # Panics
  ///
  /// Panics if `probability` is not between 0.0 and 1.0 (inclusive).
  pub fn new(probability: f64) -> Self {
    assert!(
      (0.0..=1.0).contains(&probability),
      "Probability must be between 0 and 1"
    );
    Self {
      probability,
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
