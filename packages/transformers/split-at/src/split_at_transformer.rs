use std::marker::PhantomData;
use streamweave::TransformerConfig;
use streamweave_error::ErrorStrategy;

/// A transformer that splits a stream of items at a specific index.
///
/// This transformer collects items from the input stream and splits them
/// into two groups: items before the index and items at or after the index.
#[derive(Clone)]
pub struct SplitAtTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The index at which to split the stream.
  pub index: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T> SplitAtTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `SplitAtTransformer` with the given index.
  ///
  /// # Arguments
  ///
  /// * `index` - The index at which to split the stream.
  pub fn new(index: usize) -> Self {
    Self {
      index,
      config: TransformerConfig::<T>::default(),
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
  fn test_split_at_transformer_new() {
    let transformer = SplitAtTransformer::<i32>::new(5);
    assert_eq!(transformer.index, 5);
  }

  #[test]
  fn test_split_at_transformer_with_error_strategy() {
    let transformer =
      SplitAtTransformer::<i32>::new(3).with_error_strategy(ErrorStrategy::<i32>::Skip);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_split_at_transformer_with_name() {
    let transformer = SplitAtTransformer::<i32>::new(3).with_name("test_split_at".to_string());
    assert_eq!(transformer.config.name, Some("test_split_at".to_string()));
  }

  #[test]
  fn test_split_at_transformer_clone() {
    let transformer1 = SplitAtTransformer::<i32>::new(10)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(5))
      .with_name("test_split_at".to_string());
    let transformer2 = transformer1.clone();

    assert_eq!(transformer1.index, transformer2.index);
    assert_eq!(transformer1.config.name, transformer2.config.name);
  }

  #[test]
  fn test_split_at_transformer_chaining() {
    let transformer = SplitAtTransformer::<i32>::new(10)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(3))
      .with_name("chained_split_at".to_string());

    assert_eq!(transformer.index, 10);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Retry(3)
    ));
    assert_eq!(
      transformer.config.name,
      Some("chained_split_at".to_string())
    );
  }
}
