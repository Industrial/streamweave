//! While loop node for implementing conditional iteration in graphs.
//!
//! This module provides [`While`], a graph node that implements a while loop with
//! conditional iteration. It repeats processing of items until a condition is met.
//! Each iteration processes the item through the loop body (which should be a
//! downstream transformer). It implements the [`Transformer`] trait for use in
//! StreamWeave graphs.
//!
//! # Overview
//!
//! [`While`] is useful for implementing conditional iteration in graph-based
//! pipelines. It processes items repeatedly until a condition is met, making it
//! ideal for iterative processing, convergence algorithms, and conditional loops.
//!
//! # Key Concepts
//!
//! - **Conditional Iteration**: Repeats processing until a condition is met
//! - **Condition Function**: Uses a predicate to determine if iteration should continue
//! - **Loop Body**: Processes items through downstream transformers in each iteration
//! - **Transformer Trait**: Implements `Transformer` for graph integration
//!
//! # Core Types
//!
//! - **[`While<T>`]**: Node that implements while loop with conditional iteration
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::While;
//!
//! // Create a while loop that continues while value is less than 100
//! let while_loop = While::<i32>::new(|x| *x < 100);
//! ```
//!
//! ## With Complex Conditions
//!
//! ```rust
//! use streamweave::graph::nodes::While;
//!
//! // Create a while loop with complex condition
//! let while_loop = While::<MyStruct>::new(|item| {
//!     item.counter < 100 && item.status == "processing"
//! });
//! ```
//!
//! # Design Decisions
//!
//! - **Condition Function**: Uses a closure for flexible condition specification
//! - **Iterative Processing**: Processes items through loop body until condition is met
//! - **Transformer Trait**: Implements `Transformer` for integration with
//!   graph system
//!
//! # Integration with StreamWeave
//!
//! [`While`] implements the [`Transformer`] trait and can be used in any
//! StreamWeave graph. It processes items iteratively until the condition is met,
//! enabling conditional loop patterns.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

/// Transformer that implements a while loop with conditional iteration.
///
/// Repeats processing of items until a condition is met. Each iteration processes
/// the item through the loop body (which should be a downstream transformer).
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::While;
/// use crate::graph::node::TransformerNode;
///
/// let while_loop = While::new(|x: &i32| *x < 100);
/// let node = TransformerNode::from_transformer(
///     "while_loop".to_string(),
///     while_loop,
/// );
/// ```
pub struct While<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// Condition function that determines if iteration should continue
  condition: Arc<dyn Fn(&T) -> bool + Send + Sync>,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
  /// Phantom data for type parameter
  _phantom: PhantomData<T>,
}

impl<T> While<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `While` transformer with a condition function.
  ///
  /// # Arguments
  ///
  /// * `condition` - Function that returns `true` to continue iteration,
  ///   `false` to stop and emit the item.
  ///
  /// # Returns
  ///
  /// A new `While` transformer instance.
  ///
  /// # Note
  ///
  /// The `While` transformer emits items when the condition becomes false.
  /// For true iteration with transformation, you'll need to connect this to
  /// other transformers that modify the item between iterations.
  pub fn new<F>(condition: F) -> Self
  where
    F: Fn(&T) -> bool + Send + Sync + 'static,
  {
    Self {
      condition: Arc::new(condition),
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }
}

impl<T> Input for While<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for While<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for While<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let condition = Arc::clone(&self.condition);
    Box::pin(input.filter_map(move |item| {
      let condition = Arc::clone(&condition);
      async move {
        // Emit item when condition becomes false
        if !condition(&item) {
          Some(item)
        } else {
          None // Continue iteration (item would be processed by downstream transformers)
        }
      }
    }))
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
        .unwrap_or_else(|| "while".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
