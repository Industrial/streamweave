use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use futures::Stream;
use std::marker::PhantomData;
use std::pin::Pin;

/// Defines the ordering strategy for merging multiple streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeStrategy {
  /// Process streams in order, exhaust one before moving to next.
  /// Elements from stream 0 come first, then stream 1, etc.
  Sequential,

  /// Take one element from each stream in turn (round-robin).
  /// Stream 0, Stream 1, Stream 2, Stream 0, Stream 1, ...
  RoundRobin,

  /// Process streams based on priority index (lower index = higher priority).
  /// When higher priority stream has elements, they are processed first.
  Priority,

  /// Fair interleaving using select_all (default futures behavior).
  /// Whichever stream has an element ready gets processed.
  Interleave,
}

impl Default for MergeStrategy {
  fn default() -> Self {
    Self::Interleave
  }
}

/// A transformer that merges multiple streams with configurable ordering.
///
/// This extends the basic merge functionality by supporting different
/// ordering strategies:
/// - Sequential: Exhaust streams in order
/// - RoundRobin: Take one element from each stream in turn
/// - Priority: Higher priority streams are processed first
/// - Interleave: Fair interleaving (default)
///
/// # Example
///
/// ```ignore
/// use streamweave::transformers::ordered_merge::ordered_merge_transformer::{
///     OrderedMergeTransformer, MergeStrategy
/// };
///
/// let merger = OrderedMergeTransformer::<i32>::new()
///     .with_strategy(MergeStrategy::RoundRobin);
/// ```
pub struct OrderedMergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Configuration for the transformer.
  pub config: TransformerConfig<T>,
  /// The merge strategy to use.
  pub strategy: MergeStrategy,
  /// Additional streams to merge with the input.
  pub streams: Vec<Pin<Box<dyn Stream<Item = T> + Send>>>,
  /// Phantom data for the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> Clone for OrderedMergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
      strategy: self.strategy.clone(),
      streams: Vec::new(), // Streams can't be cloned, so start with empty
      _phantom: self._phantom,
    }
  }
}

impl<T> OrderedMergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new OrderedMergeTransformer with default (Interleave) strategy.
  #[must_use]
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
      strategy: MergeStrategy::default(),
      streams: Vec::new(),
      _phantom: PhantomData,
    }
  }

  /// Sets the merge strategy.
  #[must_use]
  pub fn with_strategy(mut self, strategy: MergeStrategy) -> Self {
    self.strategy = strategy;
    self
  }

  /// Adds a stream to be merged.
  pub fn add_stream(&mut self, stream: Pin<Box<dyn Stream<Item = T> + Send>>) {
    self.streams.push(stream);
  }

  /// Adds multiple streams to be merged.
  pub fn add_streams(&mut self, streams: Vec<Pin<Box<dyn Stream<Item = T> + Send>>>) {
    self.streams.extend(streams);
  }

  /// Sets the error strategy for the transformer.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for the transformer.
  #[must_use]
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the current merge strategy.
  #[must_use]
  pub fn strategy(&self) -> &MergeStrategy {
    &self.strategy
  }

  /// Returns the number of additional streams.
  #[must_use]
  pub fn stream_count(&self) -> usize {
    self.streams.len()
  }
}

impl<T> Default for OrderedMergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_merge_strategy_default() {
    let strategy = MergeStrategy::default();
    assert_eq!(strategy, MergeStrategy::Interleave);
  }

  #[test]
  fn test_ordered_merge_transformer_new() {
    let transformer = OrderedMergeTransformer::<i32>::new();
    assert_eq!(transformer.strategy, MergeStrategy::Interleave);
    assert_eq!(transformer.stream_count(), 0);
  }

  #[test]
  fn test_ordered_merge_transformer_with_strategy() {
    let transformer =
      OrderedMergeTransformer::<i32>::new().with_strategy(MergeStrategy::RoundRobin);
    assert_eq!(transformer.strategy, MergeStrategy::RoundRobin);
  }

  #[test]
  fn test_ordered_merge_transformer_add_stream() {
    let mut transformer = OrderedMergeTransformer::<i32>::new();
    let stream = Box::pin(futures::stream::iter(vec![1, 2, 3]));
    transformer.add_stream(stream);
    assert_eq!(transformer.stream_count(), 1);
  }

  #[test]
  fn test_ordered_merge_transformer_add_streams() {
    let mut transformer = OrderedMergeTransformer::<i32>::new();
    let streams = vec![
      Box::pin(futures::stream::iter(vec![1, 2])) as Pin<Box<dyn Stream<Item = i32> + Send>>,
      Box::pin(futures::stream::iter(vec![3, 4])) as Pin<Box<dyn Stream<Item = i32> + Send>>,
    ];
    transformer.add_streams(streams);
    assert_eq!(transformer.stream_count(), 2);
  }

  #[test]
  fn test_ordered_merge_transformer_default() {
    let transformer = OrderedMergeTransformer::<i32>::default();
    assert_eq!(transformer.strategy, MergeStrategy::Interleave);
    assert_eq!(transformer.stream_count(), 0);
  }

  #[test]
  fn test_ordered_merge_transformer_with_name() {
    let transformer = OrderedMergeTransformer::<i32>::new().with_name("test_merge".to_string());
    assert_eq!(transformer.config.name, Some("test_merge".to_string()));
  }
}
