//! Builder and configuration for the RunningSumTransformer.

use crate::error::ErrorStrategy;
use crate::stateful_transformer::InMemoryStateStore;
use crate::transformer::TransformerConfig;
use std::fmt::Debug;
use std::ops::Add;
use std::sync::Arc;

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
