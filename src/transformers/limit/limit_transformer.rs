use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

/// A transformer that limits the number of items passed through the stream.
///
/// This transformer stops producing items after a specified number of items
/// have been processed, effectively truncating the stream.
pub struct LimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The maximum number of items to allow through.
  pub limit: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> LimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `LimitTransformer` with the given limit.
  ///
  /// # Arguments
  ///
  /// * `limit` - The maximum number of items to allow through the stream.
  pub fn new(limit: usize) -> Self {
    Self {
      limit,
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
  fn test_limit_transformer_new() {
    let transformer = LimitTransformer::<i32>::new(5);
    assert_eq!(transformer.limit, 5);
  }

  #[test]
  fn test_limit_transformer_with_error_strategy() {
    let transformer =
      LimitTransformer::<i32>::new(3).with_error_strategy(ErrorStrategy::<i32>::Skip);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_limit_transformer_with_name() {
    let transformer = LimitTransformer::<i32>::new(3).with_name("test_limit".to_string());
    assert_eq!(transformer.config.name, Some("test_limit".to_string()));
  }

  #[test]
  fn test_limit_transformer_chaining() {
    let transformer = LimitTransformer::<i32>::new(10)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(3))
      .with_name("chained_limit".to_string());

    assert_eq!(transformer.limit, 10);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Retry(3)
    ));
    assert_eq!(transformer.config.name, Some("chained_limit".to_string()));
  }
}
