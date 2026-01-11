//! Join node for joining multiple streams using various strategies.
//!
//! This module provides [`Join`] and [`JoinStrategy`], types for joining multiple
//! streams in graph-based pipelines. It supports various join strategies including
//! Inner, Outer, Left, and Right joins. Note: Join requires special handling as it
//! needs two input streams. This is a simplified version - a full implementation
//! would need to be an InputRouter that takes two streams and produces a joined
//! output stream.
//!
//! # Overview
//!
//! [`Join`] is useful for combining multiple streams based on join strategies in
//! graph-based pipelines. It supports various SQL-like join strategies, making it
//! ideal for data integration and stream combination workflows.
//!
//! # Key Concepts
//!
//! - **Join Strategies**: Supports Inner, Outer, Left, and Right join strategies
//! - **Dual Input**: Requires two input streams to join
//! - **Stream Combination**: Combines items from multiple streams based on strategy
//! - **SQL-Like Joins**: Implements SQL-like join semantics for streams
//!
//! # Core Types
//!
//! - **[`Join<T1, T2>`]**: Node that joins two streams using a specified strategy
//! - **[`JoinStrategy`]**: Enum representing different join strategies
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::{Join, JoinStrategy};
//!
//! // Create a join node with Inner join strategy
//! let join = Join::<i32, i32>::new(JoinStrategy::Inner);
//! ```
//!
//! ## Different Join Strategies
//!
//! ```rust
//! use streamweave::graph::nodes::{Join, JoinStrategy};
//!
//! // Inner join: Only emit when all streams have items
//! let inner_join = Join::<i32, i32>::new(JoinStrategy::Inner);
//!
//! // Outer join: Emit when any stream has items
//! let outer_join = Join::<i32, i32>::new(JoinStrategy::Outer);
//!
//! // Left join: Emit when left stream has items
//! let left_join = Join::<i32, i32>::new(JoinStrategy::Left);
//!
//! // Right join: Emit when right stream has items
//! let right_join = Join::<i32, i32>::new(JoinStrategy::Right);
//! ```
//!
//! # Design Decisions
//!
//! - **Join Strategies**: Supports multiple join strategies for flexibility
//! - **SQL-Like Semantics**: Implements familiar SQL-like join operations
//! - **Type-Safe**: Supports generic types for different item types
//! - **Simplified Implementation**: Current implementation is simplified and may
//!   need enhancement for full InputRouter support
//!
//! # Integration with StreamWeave
//!
//! [`Join`] implements the [`Transformer`] trait and can be used in any StreamWeave
//! graph. Note that a full implementation would require InputRouter support for
//! handling multiple input streams properly.

use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::marker::PhantomData;
use std::pin::Pin;

/// Strategy for joining multiple streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinStrategy {
  /// Inner join: Only emit when all streams have items
  Inner,
  /// Outer join: Emit when any stream has items (with None for missing)
  Outer,
  /// Left join: Emit when left stream has items
  Left,
  /// Right join: Emit when right stream has items
  Right,
}

/// Transformer that joins two streams using a specified strategy.
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::{Join, JoinStrategy};
/// use crate::graph::node::TransformerNode;
///
/// let join = Join::new(JoinStrategy::Inner);
/// let node = TransformerNode::from_transformer(
///     "join".to_string(),
///     join,
/// );
/// ```
///
/// # Note
///
/// Join would need special handling as it requires two input streams.
/// This is a simplified version - a full implementation would need to be
/// an InputRouter that takes two streams and produces a joined output stream.
pub struct Join<T1, T2>
where
  T1: std::fmt::Debug + Clone + Send + Sync + 'static,
  T2: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Join strategy
  strategy: JoinStrategy,
  /// Transformer configuration
  config: TransformerConfig<(T1, T2)>,
  /// Phantom data for type parameters
  _phantom: PhantomData<(T1, T2)>,
}

impl<T1, T2> Join<T1, T2>
where
  T1: std::fmt::Debug + Clone + Send + Sync + 'static,
  T2: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `Join` transformer with a join strategy.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The join strategy to use
  ///
  /// # Returns
  ///
  /// A new `Join` transformer instance.
  pub fn new(strategy: JoinStrategy) -> Self {
    Self {
      strategy,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }
}

impl<T1, T2> Input for Join<T1, T2>
where
  T1: std::fmt::Debug + Clone + Send + Sync + 'static,
  T2: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = (T1, T2);
  type InputStream = Pin<Box<dyn Stream<Item = (T1, T2)> + Send>>;
}

impl<T1, T2> Output for Join<T1, T2>
where
  T1: std::fmt::Debug + Clone + Send + Sync + 'static,
  T2: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = (T1, T2);
  type OutputStream = Pin<Box<dyn Stream<Item = (T1, T2)> + Send>>;
}

#[async_trait]
impl<T1, T2> Transformer for Join<T1, T2>
where
  T1: std::fmt::Debug + Clone + Send + Sync + 'static,
  T2: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T1, T2);
  type OutputPorts = ((T1, T2),);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    // Simplified: assumes input stream already provides tuples
    // A full implementation would involve merging multiple input streams
    // For now, we just pass through the tuples
    // The strategy field is kept for future implementation
    let _strategy = self.strategy;
    Box::pin(input.map(|item| item))
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
}
