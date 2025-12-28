use std::marker::PhantomData;
use streamweave::TransformerConfig;
use streamweave_error::ErrorStrategy;

/// A transformer that takes a specified number of items from a stream.
///
/// This transformer passes through only the first `take` items from the input
/// stream and then stops producing items.
#[derive(Clone)]
pub struct TakeTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// The number of items to take from the stream.
  pub take: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> TakeTransformer<T> {
  /// Creates a new `TakeTransformer` with the given take count.
  ///
  /// # Arguments
  ///
  /// * `take` - The number of items to take from the stream.
  pub fn new(take: usize) -> Self {
    Self {
      take,
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
  fn test_take_transformer_new() {
    let transformer = TakeTransformer::<i32>::new(5);
    assert_eq!(transformer.take, 5);
  }

  #[test]
  fn test_take_transformer_with_error_strategy() {
    let transformer =
      TakeTransformer::<i32>::new(3).with_error_strategy(ErrorStrategy::<i32>::Skip);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_take_transformer_with_name() {
    let transformer = TakeTransformer::<i32>::new(3).with_name("test_take".to_string());
    assert_eq!(transformer.config.name, Some("test_take".to_string()));
  }

  #[test]
  fn test_take_transformer_clone() {
    let transformer1 = TakeTransformer::<i32>::new(10)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(5))
      .with_name("test_take".to_string());
    let transformer2 = transformer1.clone();

    assert_eq!(transformer1.take, transformer2.take);
    assert_eq!(transformer1.config.name, transformer2.config.name);
  }

  #[test]
  fn test_take_transformer_chaining() {
    let transformer = TakeTransformer::<i32>::new(10)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(3))
      .with_name("chained_take".to_string());

    assert_eq!(transformer.take, 10);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Retry(3)
    ));
    assert_eq!(transformer.config.name, Some("chained_take".to_string()));
  }
}
