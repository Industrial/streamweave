//! Error branch router for routing Result types to success/error ports.
//!
//! This module provides [`ErrorBranch`], a router that routes `Result<T, E>` items
//! to different output ports based on success or error. It routes `Ok(item)` to
//! port 0 (success) and `Err(error)` to port 1 (error). It uses zero-copy semantics
//! by moving items directly to the appropriate port. It implements [`OutputRouter`]
//! for use in StreamWeave graphs.
//!
//! # Overview
//!
//! [`ErrorBranch`] is useful for splitting error handling in graph-based pipelines.
//! It separates successful results from errors, allowing different processing paths
//! for each case. This is essential for robust error handling patterns.
//!
//! # Key Concepts
//!
//! - **Result Routing**: Routes `Result<T, E>` items based on success/error
//! - **Dual Output Ports**: Has two output ports - one for success (0) and one
//!   for errors (1)
//! - **Zero-Copy Semantics**: Moves items directly without copying
//! - **Error Handling Pattern**: Essential for separating error paths from
//!   success paths
//!
//! # Core Types
//!
//! - **[`ErrorBranch<T, E>`]**: Router that routes Result items to success/error ports
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::graph::nodes::ErrorBranch;
//!
//! // Create an error branch router
//! let error_router = ErrorBranch::<i32, String>::new();
//! ```
//!
//! ## In a Graph
//!
//! ```rust,no_run
//! use streamweave::graph::{GraphBuilder, nodes::ErrorBranch};
//!
//! // Create a graph with error branching
//! let graph = GraphBuilder::new()
//!     .node(/* producer */)?
//!     .node(/* transformer that returns Result */)?
//!     .node(ErrorBranch::<i32, String>::new())?
//!     // Connect success port (0) to success handler
//!     // Connect error port (1) to error handler (e.g., DeadLetterQueue)
//!     .build();
//! ```
//!
//! # Design Decisions
//!
//! - **Result-Based Routing**: Uses Rust's `Result` type for type-safe error handling
//! - **Zero-Copy**: Moves items directly to appropriate ports for efficiency
//! - **Two-Port Design**: Provides clear separation between success and error paths
//! - **Router Trait**: Implements `OutputRouter` for integration with graph system
//!
//! # Integration with StreamWeave
//!
//! [`ErrorBranch`] implements the [`OutputRouter`] trait and can be used in any
//! StreamWeave graph. It routes items to different output ports based on their
//! success or error status, enabling sophisticated error handling patterns.

use crate::graph::router::OutputRouter;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::marker::PhantomData;
use std::pin::Pin;
use tokio::sync::mpsc;

/// Router that routes `Result<T, E>` items to success/error ports.
///
/// Routes `Ok(item)` to port 0 (success) and `Err(error)` to port 1 (error).
/// Uses zero-copy semantics by moving items directly to the appropriate port.
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::ErrorBranch;
/// use crate::graph::node::TransformerNode;
/// use crate::transformers::IdentityTransformer;
///
/// let error_router = ErrorBranch::<i32, String>::new();
/// let node = TransformerNode::new(
///     "split_errors".to_string(),
///     IdentityTransformer::new(),
///     error_router,
/// );
/// ```
pub struct ErrorBranch<T, E> {
  /// Phantom data for type parameters
  _phantom: PhantomData<(T, E)>,
}

impl<T, E> ErrorBranch<T, E> {
  /// Creates a new `ErrorBranch` router.
  ///
  /// # Returns
  ///
  /// A new `ErrorBranch` router instance.
  pub fn new() -> Self {
    Self {
      _phantom: PhantomData,
    }
  }
}

impl<T, E> Default for ErrorBranch<T, E> {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl<T, E> OutputRouter<Result<T, E>> for ErrorBranch<T, E>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + 'static,
{
  async fn route_stream(
    &mut self,
    stream: Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>,
  ) -> Vec<(String, Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>)> {
    // Create channels for success/error ports
    let (tx_success, rx_success) = mpsc::channel(16);
    let (tx_error, rx_error) = mpsc::channel(16);

    let mut input_stream = stream;

    // Spawn routing task - zero-copy: Results are moved to appropriate port
    tokio::spawn(async move {
      while let Some(result) = input_stream.next().await {
        match result {
          Ok(item) => {
            let _ = tx_success.send(Ok(item)).await;
          }
          Err(error) => {
            let _ = tx_error.send(Err(error)).await;
          }
        }
      }
    });

    // Create streams from receivers
    let mut rx_success_mut = rx_success;
    let mut rx_error_mut = rx_error;
    vec![
      (
        "success".to_string(),
        Box::pin(async_stream::stream! {
          while let Some(item) = rx_success_mut.recv().await {
            yield item;
          }
        }),
      ),
      (
        "error".to_string(),
        Box::pin(async_stream::stream! {
          while let Some(item) = rx_error_mut.recv().await {
            yield item;
          }
        }),
      ),
    ]
  }

  fn output_port_names(&self) -> Vec<String> {
    vec!["success".to_string(), "error".to_string()]
  }
}
