//! Group by transformer for grouping stream items by a key function.
//!
//! This module provides [`GroupByTransformer`], a transformer that groups stream
//! items by a key extracted from each item using a key function. It groups items
//! from the input stream based on the extracted key, producing groups of items
//! with the same key. It implements the [`Transformer`] trait for use in StreamWeave
//! pipelines and graphs.
//!
//! # Overview
//!
//! [`GroupByTransformer`] is useful for grouping items in StreamWeave pipelines.
//! It processes items and groups them by a key extracted using a key function,
//! producing groups of items with the same key. Groups are emitted as tuples
//! `(key, Vec<item>)` in sorted order by key.
//!
//! # Key Concepts
//!
//! - **Key Extraction**: Uses a key function to extract grouping keys from items
//! - **Grouping**: Groups items by their extracted keys
//! - **Sorted Output**: Groups are emitted in sorted order by key for deterministic
//!   output
//! - **Transformer Trait**: Implements `Transformer` for pipeline integration
//!
//! # Core Types
//!
//! - **[`GroupByTransformer<F, T, K>`]**: Transformer that groups items by a key
//!   function
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::GroupByTransformer;
//!
//! // Group items by a key extracted from each item
//! let transformer = GroupByTransformer::new(|item: &MyStruct| item.category.clone());
//! ```
//!
//! ## With Complex Key Function
//!
//! ```rust
//! use streamweave::transformers::GroupByTransformer;
//!
//! // Group items by a composite key
//! let transformer = GroupByTransformer::new(|item: &MyStruct| {
//!     (item.category.clone(), item.status.clone())
//! });
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::GroupByTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a group by transformer with error handling
//! let transformer = GroupByTransformer::new(|item: &MyStruct| item.category.clone())
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("category-grouper".to_string());
//! ```
//!
//! # Design Decisions
//!
//! - **Key Function**: Uses a closure for flexible key extraction
//! - **Sorted Output**: Sorts groups by key for deterministic output
//! - **HashMap-Based**: Uses HashMap for efficient grouping
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   pipeline system
//!
//! # Integration with StreamWeave
//!
//! [`GroupByTransformer`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave pipeline or graph. It supports the standard error handling strategies
//! and configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_stream;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::pin::Pin;

/// A transformer that groups stream items by a key function.
///
/// This transformer groups items from the input stream based on a key extracted
/// by the provided function, producing groups of items with the same key.
#[derive(Clone)]
pub struct GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  /// The function used to extract the grouping key from each item.
  pub key_fn: F,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the item type parameter.
  pub _phantom_t: PhantomData<T>,
  /// Phantom data to track the key type parameter.
  pub _phantom_k: PhantomData<K>,
}

impl<F, T, K> GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Send + Clone + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  /// Creates a new `GroupByTransformer` with the given key function.
  ///
  /// # Arguments
  ///
  /// * `key_fn` - The function to extract the grouping key from each item.
  pub fn new(key_fn: F) -> Self {
    Self {
      key_fn,
      config: TransformerConfig::default(),
      _phantom_t: PhantomData,
      _phantom_k: PhantomData,
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

impl<F, T, K> Input for GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Clone + Send + Sync + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<F, T, K> Output for GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Clone + Send + Sync + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  type Output = (K, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (K, Vec<T>)> + Send>>;
}

#[async_trait]
impl<F, T, K> Transformer for GroupByTransformer<F, T, K>
where
  F: Fn(&T) -> K + Clone + Send + Sync + 'static,
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = ((K, Vec<T>),);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let key_fn = self.key_fn.clone();
    Box::pin(async_stream::stream! {
        let mut groups = HashMap::new();
        let mut input = input;

        while let Some(item) = input.next().await {
            let key = key_fn(&item);
            groups.entry(key).or_insert_with(Vec::new).push(item);
        }

        // Sort groups by key for deterministic output
        let mut groups = groups.into_iter().collect::<Vec<_>>();
        groups.sort_by_key(|(k, _)| k.clone());

        for group in groups {
            yield group;
        }
    })
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
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "group_by_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "group_by_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
