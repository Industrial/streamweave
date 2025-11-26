//! Join transformer for combining two streams based on keys.
//!
//! This transformer implements inner join semantics, combining elements from
//! two streams when their keys match within a configurable time window.
//!
//! # Example
//!
//! ```rust,ignore
//! use streamweave::transformers::join::join_transformer::JoinTransformer;
//!
//! // Join orders with users by user_id
//! let join = JoinTransformer::new(
//!     |order: &Order| order.user_id,
//!     |user: &User| user.id,
//! );
//! ```

use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use futures::Stream;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::pin::Pin;
use std::time::Duration;

/// A transformer that joins two streams based on matching keys.
///
/// The join operates within a configurable time window to limit memory usage.
/// Elements are buffered from both streams and matched when keys are equal.
pub struct JoinTransformer<L, R, K, LF, RF>
where
  L: std::fmt::Debug + Clone + Send + Sync + 'static,
  R: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: Hash + Eq + Clone + Send + Sync + 'static,
  LF: Fn(&L) -> K + Clone + Send + Sync + 'static,
  RF: Fn(&R) -> K + Clone + Send + Sync + 'static,
{
  /// Function to extract key from left stream elements.
  pub left_key_fn: LF,
  /// Function to extract key from right stream elements.
  pub right_key_fn: RF,
  /// Configuration for the transformer.
  pub config: TransformerConfig<L>,
  /// Window duration for join buffer.
  pub window_duration: Duration,
  /// The right stream to join with.
  pub right_stream: Option<Pin<Box<dyn Stream<Item = R> + Send>>>,
  /// Phantom data for type parameters.
  _phantom: PhantomData<(L, R, K)>,
}

impl<L, R, K, LF, RF> Clone for JoinTransformer<L, R, K, LF, RF>
where
  L: std::fmt::Debug + Clone + Send + Sync + 'static,
  R: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: Hash + Eq + Clone + Send + Sync + 'static,
  LF: Fn(&L) -> K + Clone + Send + Sync + 'static,
  RF: Fn(&R) -> K + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      left_key_fn: self.left_key_fn.clone(),
      right_key_fn: self.right_key_fn.clone(),
      config: self.config.clone(),
      window_duration: self.window_duration,
      right_stream: None, // Streams can't be cloned
      _phantom: PhantomData,
    }
  }
}

impl<L, R, K, LF, RF> JoinTransformer<L, R, K, LF, RF>
where
  L: std::fmt::Debug + Clone + Send + Sync + 'static,
  R: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: Hash + Eq + Clone + Send + Sync + 'static,
  LF: Fn(&L) -> K + Clone + Send + Sync + 'static,
  RF: Fn(&R) -> K + Clone + Send + Sync + 'static,
{
  /// Creates a new join transformer with the given key extraction functions.
  pub fn new(left_key_fn: LF, right_key_fn: RF) -> Self {
    Self {
      left_key_fn,
      right_key_fn,
      config: TransformerConfig::default(),
      window_duration: Duration::from_secs(60), // Default 1 minute window
      right_stream: None,
      _phantom: PhantomData,
    }
  }

  /// Sets the right stream to join with.
  pub fn with_right_stream(mut self, stream: Pin<Box<dyn Stream<Item = R> + Send>>) -> Self {
    self.right_stream = Some(stream);
    self
  }

  /// Sets the window duration for the join buffer.
  pub fn with_window_duration(mut self, duration: Duration) -> Self {
    self.window_duration = duration;
    self
  }

  /// Sets the error strategy for the transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<L>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name of the transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

/// Result of a join operation - a pair of matched elements.
#[derive(Debug, Clone)]
pub struct JoinResult<L, R> {
  /// The element from the left stream.
  pub left: L,
  /// The element from the right stream.
  pub right: R,
}

impl<L, R> JoinResult<L, R> {
  /// Creates a new join result.
  pub fn new(left: L, right: R) -> Self {
    Self { left, right }
  }
}

/// Internal state for the join operation.
#[derive(Debug)]
pub(crate) struct JoinState<L, R, K>
where
  K: Hash + Eq,
{
  /// Buffer of left elements by key.
  pub left_buffer: HashMap<K, Vec<L>>,
  /// Buffer of right elements by key.
  pub right_buffer: HashMap<K, Vec<R>>,
}

impl<L, R, K> Default for JoinState<L, R, K>
where
  K: Hash + Eq,
{
  fn default() -> Self {
    Self {
      left_buffer: HashMap::new(),
      right_buffer: HashMap::new(),
    }
  }
}

impl<L, R, K> JoinState<L, R, K>
where
  L: Clone,
  R: Clone,
  K: Hash + Eq + Clone,
{
  /// Adds a left element and returns any matching join results.
  pub fn add_left(&mut self, key: K, element: L) -> Vec<JoinResult<L, R>> {
    let mut results = Vec::new();

    // Check for matches in right buffer
    if let Some(right_elements) = self.right_buffer.get(&key) {
      for right in right_elements {
        results.push(JoinResult::new(element.clone(), right.clone()));
      }
    }

    // Add to left buffer
    self.left_buffer.entry(key).or_default().push(element);

    results
  }

  /// Adds a right element and returns any matching join results.
  pub fn add_right(&mut self, key: K, element: R) -> Vec<JoinResult<L, R>> {
    let mut results = Vec::new();

    // Check for matches in left buffer
    if let Some(left_elements) = self.left_buffer.get(&key) {
      for left in left_elements {
        results.push(JoinResult::new(left.clone(), element.clone()));
      }
    }

    // Add to right buffer
    self.right_buffer.entry(key).or_default().push(element);

    results
  }

  /// Clears both buffers.
  #[allow(dead_code)]
  pub fn clear(&mut self) {
    self.left_buffer.clear();
    self.right_buffer.clear();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_join_state_basic() {
    let mut state: JoinState<i32, String, i32> = JoinState::default();

    // Add left element first
    let results = state.add_left(1, 100);
    assert!(results.is_empty()); // No match yet

    // Add matching right element
    let results = state.add_right(1, "one".to_string());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].left, 100);
    assert_eq!(results[0].right, "one");

    // Add another left with same key
    let results = state.add_left(1, 101);
    assert_eq!(results.len(), 1); // Matches existing right
    assert_eq!(results[0].left, 101);
    assert_eq!(results[0].right, "one");
  }

  #[test]
  fn test_join_state_no_match() {
    let mut state: JoinState<i32, String, i32> = JoinState::default();

    let results = state.add_left(1, 100);
    assert!(results.is_empty());

    let results = state.add_right(2, "two".to_string());
    assert!(results.is_empty()); // Different key, no match
  }

  #[test]
  fn test_join_state_multiple_matches() {
    let mut state: JoinState<i32, String, i32> = JoinState::default();

    state.add_left(1, 100);
    state.add_left(1, 101);
    state.add_right(1, "a".to_string());

    let results = state.add_right(1, "b".to_string());
    // Should match both left elements
    assert_eq!(results.len(), 2);
  }

  #[test]
  fn test_join_result() {
    let result = JoinResult::new(42, "answer");
    assert_eq!(result.left, 42);
    assert_eq!(result.right, "answer");
  }

  #[test]
  fn test_join_transformer_builder() {
    let transformer: JoinTransformer<i32, String, i32, _, _> =
      JoinTransformer::new(|x: &i32| *x, |s: &String| s.len() as i32)
        .with_window_duration(Duration::from_secs(30))
        .with_name("test_join".to_string());

    assert_eq!(transformer.window_duration, Duration::from_secs(30));
    assert_eq!(transformer.config.name, Some("test_join".to_string()));
  }

  #[test]
  fn test_join_transformer_clone() {
    let transformer: JoinTransformer<i32, String, i32, _, _> =
      JoinTransformer::new(|x: &i32| *x, |s: &String| s.len() as i32)
        .with_name("original".to_string());

    let cloned = transformer.clone();
    assert_eq!(cloned.config.name, Some("original".to_string()));
    assert!(cloned.right_stream.is_none()); // Streams can't be cloned
  }
}
