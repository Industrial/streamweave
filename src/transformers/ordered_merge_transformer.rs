//! # Ordered Merge Transformer
//!
//! Transformer that merges multiple input streams into a single output stream
//! using configurable ordering strategies. This enables controlled interleaving
//! of items from multiple sources with various merge policies.
//!
//! ## Overview
//!
//! The Ordered Merge Transformer provides:
//!
//! - **Multi-Stream Merging**: Merges items from multiple input streams
//! - **Ordering Strategies**: Configurable merge strategies (Sequential, RoundRobin, Priority, Interleave)
//! - **Controlled Interleaving**: Deterministic or fair interleaving based on strategy
//! - **Type Generic**: Works with any `Send + Sync + Clone` type
//! - **Error Handling**: Configurable error strategies
//!
//! ## Input/Output
//!
//! - **Input**: Multiple streams of `Message<T>`
//! - **Output**: `Message<T>` - Merged items from all input streams
//!
//! ## Merge Strategies
//!
//! - **Sequential**: Process streams in order, exhaust one before moving to next
//! - **RoundRobin**: Take one element from each stream in turn
//! - **Priority**: Process streams based on priority index (lower = higher priority)
//! - **Interleave**: Fair interleaving (whichever stream has an element ready)
//!
//! ## Example
//!
//! ```rust
//! use crate::transformers::{OrderedMergeTransformer, MergeStrategy};
//!
//! let transformer = OrderedMergeTransformer::new(MergeStrategy::RoundRobin);
//! // Merges multiple streams in round-robin fashion
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_stream::stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt, stream::select_all};
use std::marker::PhantomData;
use std::pin::Pin;

/// Defines the ordering strategy for merging multiple streams.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
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
  #[default]
  Interleave,
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
/// use crate::transformers::{
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

impl<T> Input for OrderedMergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for OrderedMergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for OrderedMergeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let strategy = self.strategy.clone();
    let mut all_streams = vec![input];
    all_streams.extend(std::mem::take(&mut self.streams));

    match strategy {
      MergeStrategy::Interleave => {
        // Use select_all for fair interleaving
        Box::pin(select_all(all_streams))
      }

      MergeStrategy::Sequential => {
        // Process streams in order, exhaust each before moving to next
        Box::pin(stream! {
            for mut stream in all_streams {
                while let Some(item) = stream.next().await {
                    yield item;
                }
            }
        })
      }

      MergeStrategy::RoundRobin => {
        // Take one element from each stream in turn
        Box::pin(stream! {
            let mut streams: Vec<_> = all_streams.into_iter().map(Some).collect();
            let mut active_count = streams.len();

            loop {
                if active_count == 0 {
                    break;
                }

                for stream_option in &mut streams {
                    if let Some(stream) = stream_option {
                        match stream.next().await {
                            Some(item) => yield item,
                            None => {
                                *stream_option = None;
                                active_count -= 1;
                            }
                        }
                    }
                }
            }
        })
      }

      MergeStrategy::Priority => {
        // Process streams based on priority (lower index = higher priority)
        // When higher priority stream has elements, process them first
        Box::pin(stream! {
            let mut streams: Vec<_> = all_streams.into_iter().map(Some).collect();
            let mut active_count = streams.len();

            loop {
                if active_count == 0 {
                    break;
                }

                // Try to get from highest priority stream first
                let mut got_item = false;
                for stream_option in &mut streams {
                    if let Some(stream) = stream_option {
                        match stream.next().await {
                            Some(item) => {
                                yield item;
                                got_item = true;
                                break; // After yielding from highest priority, check again
                            }
                            None => {
                                *stream_option = None;
                                active_count -= 1;
                            }
                        }
                    }
                }

                // If we didn't get any item and still have active streams,
                // they're all waiting for async operations - this shouldn't happen
                // with synchronous stream::iter, but let's be safe
                if !got_item && active_count > 0 {
                    // All active streams are blocked, yield control
                    tokio::task::yield_now().await;
                }
            }
        })
      }
    }
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "ordered_merge_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
