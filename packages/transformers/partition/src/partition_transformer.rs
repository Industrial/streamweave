use std::marker::PhantomData;
use streamweave_core::TransformerConfig;
use streamweave_error::ErrorStrategy;

/// A transformer that partitions a stream into two streams based on a predicate.
///
/// This transformer splits the input stream into two output streams: one for items
/// that match the predicate and one for items that don't.
pub struct PartitionTransformer<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The predicate function used to partition items.
  pub predicate: F,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<F, T> PartitionTransformer<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `PartitionTransformer` with the given predicate.
  ///
  /// # Arguments
  ///
  /// * `predicate` - The function used to determine which partition an item belongs to.
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
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
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_partition_transformer_new() {
    let transformer = PartitionTransformer::<_, i32>::new(|x: &i32| *x > 0);
    // Test that it can be created
    assert!((transformer.predicate)(&1));
    assert!(!(transformer.predicate)(&-1));
  }

  #[test]
  fn test_partition_transformer_with_error_strategy() {
    let transformer = PartitionTransformer::<_, i32>::new(|x: &i32| *x > 0)
      .with_error_strategy(ErrorStrategy::<i32>::Skip);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_partition_transformer_with_name() {
    let transformer =
      PartitionTransformer::<_, i32>::new(|x: &i32| *x > 0).with_name("test_partition".to_string());
    assert_eq!(transformer.config.name, Some("test_partition".to_string()));
  }

  #[test]
  fn test_partition_transformer_chaining() {
    let transformer = PartitionTransformer::<_, i32>::new(|x: &i32| *x > 0)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(3))
      .with_name("chained_partition".to_string());

    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Retry(3)
    ));
    assert_eq!(
      transformer.config.name,
      Some("chained_partition".to_string())
    );
  }
}
