//! Router that routes `Result<T, E>` items to success/error ports.
//!
//! Routes `Ok(item)` to port 0 (success) and `Err(error)` to port 1 (error).
//! Uses zero-copy semantics by moving items directly to the appropriate port.

use crate::router::OutputRouter;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::marker::PhantomData;
use std::pin::Pin;

/// Router that routes `Result<T, E>` items to success/error ports.
///
/// Routes `Ok(item)` to port 0 (success) and `Err(error)` to port 1 (error).
/// Uses zero-copy semantics by moving items directly to the appropriate port.
///
/// # Example
///
/// ```rust
/// use streamweave::graph::control_flow::ErrorBranch;
/// use streamweave::graph::node::TransformerNode;
/// use streamweave_transformers::IdentityTransformer;
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
  ) -> Vec<(usize, Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>)> {
    use tokio::sync::mpsc;

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
        0,
        Box::pin(async_stream::stream! {
          while let Some(item) = rx_success_mut.recv().await {
            yield item;
          }
        }),
      ),
      (
        1,
        Box::pin(async_stream::stream! {
          while let Some(item) = rx_error_mut.recv().await {
            yield item;
          }
        }),
      ),
    ]
  }

  fn output_ports(&self) -> Vec<usize> {
    vec![0, 1] // success port, error port
  }
}
