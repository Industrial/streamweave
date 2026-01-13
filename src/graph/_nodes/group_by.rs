//! GroupBy node for grouping items by a key function.
//!
//! This module provides [`GroupBy`], a graph node that groups items by a key function.
//! It wraps `GroupByTransformer` to provide a graph node implementation. The transformer
//! provides deterministic sorted output by key, making it ideal for aggregation and
//! grouping operations in graph-based pipelines.
//!
//! # Overview
//!
//! [`GroupBy`] is useful for grouping items in graph-based pipelines based on a
//! computed key. It processes items and groups them by their key values, outputting
//! groups in sorted order for deterministic processing.
//!
//! # Key Concepts
//!
//! - **Key Extraction**: Uses a key function to extract grouping keys from items
//! - **Deterministic Output**: Outputs groups in sorted order by key
//! - **Grouping**: Groups items with the same key together
//! - **Hash-Based**: Uses hash maps internally for efficient grouping
//!
//! # Core Types
//!
//! - **[`GroupBy<T, K>`]**: Node that groups items by a key function
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::GroupBy;
//!
//! // Group items by a key extracted from the item
//! let group_by = GroupBy::new(|item: &(String, i32)| item.0.clone());
//! ```
//!
//! # Design Decisions
//!
//! - **Key Function**: Uses a closure for flexible key extraction from items
//! - **Sorted Output**: Provides deterministic sorted output by key for
//!   reproducible results
//! - **Hash-Based Grouping**: Uses hash maps for efficient grouping operations
//! - **Type-Safe**: Supports generic types for items and keys
//!
//! # Integration with StreamWeave
//!
//! [`GroupBy`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It supports the standard error handling strategies and
//! configuration options provided by [`TransformerConfig`].

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};
use crate::transformers::GroupByTransformer;
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::hash::Hash;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

/// Transformer that groups items by a key function.
///
/// This is a wrapper around `GroupByTransformer` that maintains the `Arc<dyn Fn>` interface
/// for graph nodes while delegating the actual grouping logic to the transformer.
/// The transformer provides deterministic sorted output by key.
///
/// # Example
///
/// ```rust
/// use crate::graph::nodes::GroupBy;
/// use crate::graph::nodes::TransformerNode;
///
/// let group_by = GroupBy::new(|item: &(String, i32)| item.0.clone());
/// let node = TransformerNode::from_transformer(
///     "group".to_string(),
///     group_by,
/// );
/// ```
pub struct GroupBy<
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
> {
  /// Function to extract key from item
  key_fn: Arc<dyn Fn(&T) -> K + Send + Sync>,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
  /// Phantom data for type parameters
  _phantom: PhantomData<(T, K)>,
}

impl<T, K> GroupBy<T, K>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  /// Creates a new `GroupBy` transformer.
  ///
  /// # Arguments
  ///
  /// * `key_fn` - Function that extracts a key from each item
  ///
  /// # Returns
  ///
  /// A new `GroupBy` transformer instance.
  ///
  /// # Note
  ///
  /// The key type `K` must implement `Ord` in addition to `Hash` and `Eq`
  /// to enable deterministic sorted output (groups are sorted by key).
  pub fn new<F>(key_fn: F) -> Self
  where
    F: Fn(&T) -> K + Send + Sync + 'static,
  {
    Self {
      key_fn: Arc::new(key_fn),
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }
}

impl<T, K> Input for GroupBy<T, K>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T, K> Output for GroupBy<T, K>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  type Output = (K, Vec<T>);
  type OutputStream = Pin<Box<dyn Stream<Item = (K, Vec<T>)> + Send>>;
}

#[async_trait]
impl<T, K> Transformer for GroupBy<T, K>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + Ord + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = ((K, Vec<T>),);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    // Create a GroupByTransformer with a closure that calls our Arc function.
    // This delegates to the transformer's implementation which provides sorted output.
    //
    // Note: We create a new transformer instance here, but this is called once per
    // stream transformation (not per item). The overhead is minimal - just creating
    // a struct with a closure and phantom data. The real work (processing the stream)
    // happens in the transformer's transform() method.
    //
    // We can't easily cache the transformer instance due to type system constraints:
    // GroupByTransformer requires F: Clone, but we use Arc<dyn Fn> which can't be
    // converted to a cloneable function type without additional overhead.
    let key_fn = Arc::clone(&self.key_fn);
    let mut transformer = GroupByTransformer::new(move |item: &T| key_fn(item));

    // Copy configuration from self to transformer
    transformer.set_config_impl(self.config.clone());

    // Delegate to the transformer's transform method
    transformer.transform(input).await
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy() {
      crate::error::ErrorStrategy::Stop => ErrorAction::Stop,
      crate::error::ErrorStrategy::Skip => ErrorAction::Skip,
      crate::error::ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      crate::error::ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "group_by".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
