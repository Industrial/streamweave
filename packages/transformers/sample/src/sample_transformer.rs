use std::marker::PhantomData;
use streamweave_core::TransformerConfig;
use streamweave_error::ErrorStrategy;

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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sample_transformer_new() {
    let transformer = SampleTransformer::<i32>::new(0.5);
    assert_eq!(transformer.probability, 0.5);
  }

  #[test]
  fn test_sample_transformer_new_zero() {
    let transformer = SampleTransformer::<i32>::new(0.0);
    assert_eq!(transformer.probability, 0.0);
  }

  #[test]
  fn test_sample_transformer_new_one() {
    let transformer = SampleTransformer::<i32>::new(1.0);
    assert_eq!(transformer.probability, 1.0);
  }

  #[test]
  #[should_panic(expected = "Probability must be between 0 and 1")]
  fn test_sample_transformer_new_invalid_negative() {
    let _ = SampleTransformer::<i32>::new(-0.1);
  }

  #[test]
  #[should_panic(expected = "Probability must be between 0 and 1")]
  fn test_sample_transformer_new_invalid_above_one() {
    let _ = SampleTransformer::<i32>::new(1.1);
  }

  #[test]
  fn test_sample_transformer_with_error_strategy() {
    let transformer =
      SampleTransformer::<i32>::new(0.5).with_error_strategy(ErrorStrategy::<i32>::Skip);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_sample_transformer_with_name() {
    let transformer = SampleTransformer::<i32>::new(0.5).with_name("test_sample".to_string());
    assert_eq!(transformer.config.name, Some("test_sample".to_string()));
  }

  #[test]
  fn test_sample_transformer_chaining() {
    let transformer = SampleTransformer::<i32>::new(0.75)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(3))
      .with_name("chained_sample".to_string());
    assert_eq!(transformer.probability, 0.75);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Retry(3)
    ));
    assert_eq!(transformer.config.name, Some("chained_sample".to_string()));
  }
}
