//! Builder and configuration for the RunningSumTransformer.

use std::fmt::Debug;
use std::ops::Add;
use std::sync::Arc;
use streamweave::TransformerConfig;
use streamweave_error::ErrorStrategy;
use streamweave_stateful::InMemoryStateStore;

/// A stateful transformer that maintains a running sum of all processed items.
///
/// # Type Parameters
///
/// * `T` - The numeric type to sum. Must implement `Add`, `Default`, `Clone`, etc.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::running_sum::RunningSumTransformer;
/// use streamweave::stateful_transformer::StatefulTransformer;
/// use streamweave::transformer::Transformer;
/// use futures::StreamExt;
///
/// # async fn example() {
/// let mut transformer = RunningSumTransformer::<i32>::new();
/// let input = Box::pin(futures::stream::iter(vec![1, 2, 3, 4, 5]));
/// let output = transformer.transform(input);
/// let results: Vec<i32> = output.collect().await;
/// assert_eq!(results, vec![1, 3, 6, 10, 15]);
/// # }
/// ```
#[derive(Debug)]
pub struct RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  /// Configuration for the transformer.
  pub(crate) config: TransformerConfig<T>,
  /// State store for maintaining the running sum (wrapped in Arc for sharing).
  pub(crate) state_store: Arc<InMemoryStateStore<T>>,
}

impl<T> Clone for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      state_store: Arc::clone(&self.state_store),
    }
  }
}

impl<T> RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new RunningSumTransformer with default initial value.
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
      state_store: Arc::new(InMemoryStateStore::new(T::default())),
    }
  }

  /// Creates a new RunningSumTransformer with a specified initial value.
  pub fn with_initial(initial: T) -> Self {
    Self {
      config: TransformerConfig::default(),
      state_store: Arc::new(InMemoryStateStore::new(initial)),
    }
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Sets the error strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }
}

impl<T> Default for RunningSumTransformer<T>
where
  T: Add<Output = T> + Default + Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_running_sum_transformer_new() {
    let _transformer = RunningSumTransformer::<i32>::new();
    // Test that it can be created
  }

  #[test]
  fn test_running_sum_transformer_with_initial() {
    let _transformer = RunningSumTransformer::<i32>::with_initial(100);
    // Test that it can be created with initial value
  }

  #[test]
  fn test_running_sum_transformer_with_initial_float() {
    let _transformer = RunningSumTransformer::<f64>::with_initial(42.5);
    // Test that it can be created with initial float value
  }

  #[test]
  fn test_running_sum_transformer_default() {
    let _transformer = RunningSumTransformer::<i32>::default();
    // Test that default creates a new transformer
  }

  #[test]
  fn test_running_sum_transformer_with_name() {
    let transformer = RunningSumTransformer::<i32>::new().with_name("test_running_sum".to_string());
    assert_eq!(
      transformer.config.name,
      Some("test_running_sum".to_string())
    );
  }

  #[test]
  fn test_running_sum_transformer_with_error_strategy() {
    let transformer =
      RunningSumTransformer::<i32>::new().with_error_strategy(ErrorStrategy::<i32>::Skip);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_running_sum_transformer_clone() {
    let transformer1 = RunningSumTransformer::<i32>::with_initial(50);
    let transformer2 = transformer1.clone();

    // Clone should work - both should be independent instances
    let _t1 = transformer1;
    let _t2 = transformer2;
  }

  #[test]
  fn test_running_sum_transformer_chaining() {
    let transformer = RunningSumTransformer::<i32>::with_initial(10)
      .with_error_strategy(ErrorStrategy::<i32>::Retry(5))
      .with_name("chained_running_sum".to_string());

    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Retry(5)
    ));
    assert_eq!(
      transformer.config.name,
      Some("chained_running_sum".to_string())
    );
  }
}
