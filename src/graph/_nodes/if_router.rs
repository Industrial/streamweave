//! If router for conditional routing based on predicates.
//!
//! This module provides [`If`], a router that routes items based on a predicate
//! function (if/else pattern). It routes items to port 0 if the predicate returns
//! `true`, otherwise to port 1. It uses zero-copy semantics by moving items directly
//! to the appropriate port. It implements [`OutputRouter`] for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`If`] is useful for conditional routing in graph-based pipelines. It splits
//! streams into two paths based on a condition, enabling different processing paths
//! for items that meet or don't meet the condition. This is essential for conditional
//! processing patterns.
//!
//! # Key Concepts
//!
//! - **Conditional Routing**: Routes items based on a predicate function
//! - **Two Output Ports**: Has two output ports - port 0 for true, port 1 for false
//! - **Zero-Copy Semantics**: Moves items directly without copying
//! - **If/Else Pattern**: Implements the classic if/else branching pattern
//!
//! # Core Types
//!
//! - **[`If<O>`]**: Router that routes items based on a predicate
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::If;
//!
//! // Route items based on a condition
//! let if_router = If::new(|x: &i32| *x % 2 == 0);
//! ```
//!
//! ## In a Graph
//!
//! ```rust,no_run
//! use streamweave::graph::{GraphBuilder, nodes::If};
//!
//! // Create a graph with conditional routing
//! let graph = GraphBuilder::new()
//!     .node(/* producer */)?
//!     .node(If::new(|x: &i32| *x > 10))?
//!     // Connect port 0 (true) to high-value processor
//!     // Connect port 1 (false) to low-value processor
//!     .build();
//! ```
//!
//! # Design Decisions
//!
//! - **Predicate Function**: Uses a closure for flexible condition evaluation
//! - **Zero-Copy**: Moves items directly to appropriate ports for efficiency
//! - **Two-Port Design**: Provides clear separation between true and false paths
//! - **Router Trait**: Implements `OutputRouter` for integration with graph system
//!
//! # Integration with StreamWeave
//!
//! [`If`] implements the [`OutputRouter`] trait and can be used in any StreamWeave
//! graph. It routes items to different output ports based on predicate evaluation,
//! enabling conditional processing patterns.

use crate::graph::router::OutputRouter;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Conditional router that routes items based on a predicate (if/else).
///
/// Routes items to port 0 if the predicate returns `true`, otherwise to port 1.
/// Uses zero-copy semantics by moving items directly to the appropriate port.
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::If;
/// use crate::graph::node::TransformerNode;
/// use crate::transformers::IdentityTransformer;
///
/// let if_router = If::new(|x: &i32| *x % 2 == 0);
/// let node = TransformerNode::new(
///     "split".to_string(),
///     IdentityTransformer::new(),
///     if_router,
/// );
/// ```
pub struct If<O> {
  /// Predicate function that determines routing
  predicate: Arc<dyn Fn(&O) -> bool + Send + Sync>,
  /// Phantom data for type parameter
  _phantom: PhantomData<O>,
}

impl<O> If<O>
where
  O: Send + Sync + 'static,
{
  /// Creates a new `If` router with a predicate function.
  ///
  /// # Arguments
  ///
  /// * `predicate` - Function that takes a reference to an item and returns `true`
  ///   to route to port 0, or `false` to route to port 1.
  ///
  /// # Returns
  ///
  /// A new `If` router instance.
  pub fn new<F>(predicate: F) -> Self
  where
    F: Fn(&O) -> bool + Send + Sync + 'static,
  {
    Self {
      predicate: Arc::new(predicate),
      _phantom: PhantomData,
    }
  }
}

#[async_trait]
impl<O> OutputRouter<O> for If<O>
where
  O: Send + Sync + Clone + 'static,
{
  async fn route_stream(
    &mut self,
    stream: Pin<Box<dyn Stream<Item = O> + Send>>,
  ) -> Vec<(String, Pin<Box<dyn Stream<Item = O> + Send>>)> {
    // Create channels for true/false ports
    let (tx_true, rx_true) = mpsc::channel(16);
    let (tx_false, rx_false) = mpsc::channel(16);

    let predicate = Arc::clone(&self.predicate);
    let mut input_stream = stream;

    // Spawn routing task - zero-copy: items are moved to the appropriate port
    tokio::spawn(async move {
      while let Some(item) = input_stream.next().await {
        if predicate(&item) {
          let _ = tx_true.send(item).await;
        } else {
          let _ = tx_false.send(item).await;
        }
      }
    });

    // Create streams from receivers
    let mut rx_true_mut = rx_true;
    let mut rx_false_mut = rx_false;
    vec![
      (
        "true".to_string(),
        Box::pin(async_stream::stream! {
          while let Some(item) = rx_true_mut.recv().await {
            yield item;
          }
        }),
      ),
      (
        "false".to_string(),
        Box::pin(async_stream::stream! {
          while let Some(item) = rx_false_mut.recv().await {
            yield item;
          }
        }),
      ),
    ]
  }

  fn output_port_names(&self) -> Vec<String> {
    vec!["true".to_string(), "false".to_string()]
  }
}
