use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;

/// A transformer that groups items from a stream into chunks of a specified size.
///
/// This transformer collects items into vectors of up to `size` items each.
pub struct ChunkTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The maximum size of each chunk.
  pub size: usize,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the input type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> ChunkTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `ChunkTransformer` with the given chunk size.
  ///
  /// # Arguments
  ///
  /// * `size` - The maximum number of items in each chunk.
  pub fn new(size: usize) -> Self {
    Self {
      size,
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
  fn test_chunk_transformer_new() {
    let transformer = ChunkTransformer::<i32>::new(5);
    assert_eq!(transformer.size, 5);
  }

  #[test]
  fn test_chunk_transformer_with_error_strategy() {
    let transformer =
      ChunkTransformer::<i32>::new(3).with_error_strategy(ErrorStrategy::<i32>::Skip);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_chunk_transformer_with_name() {
    let transformer = ChunkTransformer::<i32>::new(3).with_name("test_chunk".to_string());
    assert_eq!(transformer.config.name, Some("test_chunk".to_string()));
  }

  #[test]
  fn test_chunk_transformer_chaining() {
    let transformer = ChunkTransformer::<i32>::new(10)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(3))
      .with_name("chained_chunk".to_string());

    assert_eq!(transformer.size, 10);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Retry(3)
    ));
    assert_eq!(transformer.config.name, Some("chained_chunk".to_string()));
  }
}
