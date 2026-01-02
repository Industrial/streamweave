//! # Stateful Node Support
//!
//! This module provides support for stateful processing nodes in the graph API.
//! Stateful nodes maintain state across stream items, enabling use cases like:
//!
//! - Running aggregations (sum, average, count)
//! - Session management
//! - Pattern detection across items
//! - Stateful windowing operations
//!
//! Stateful nodes wrap stateful transformers and provide access to their state
//! through the graph API.
//!
//! ## Architecture
//!
//! Stateful nodes access transformers through `Arc<tokio::sync::Mutex<T>>`, which
//! requires runtime detection to handle both sync and async contexts. The state
//! access methods use runtime detection to safely lock the mutex and access
//! transformer state.

use crate::node::TransformerNode;
use crate::traits::NodeTrait;
use streamweave::Transformer;
use streamweave_stateful::{StateError, StateResult, StatefulTransformer};

/// Trait for accessing state from stateful nodes.
///
/// This trait allows the graph execution engine and other components to
/// interact with the state of stateful nodes.
pub trait StatefulNode: NodeTrait {
  /// Returns the current state of this node, if it is stateful.
  ///
  /// # Returns
  ///
  /// `Some(state)` if the node is stateful and has state, `None` otherwise.
  ///
  /// # Errors
  ///
  /// Returns an error if state access fails (e.g., lock poisoning).
  fn get_state(&self) -> StateResult<Option<Box<dyn std::any::Any + Send + Sync>>>;

  /// Sets the state of this node, if it is stateful.
  ///
  /// # Arguments
  ///
  /// * `state` - The new state value (must match the node's state type)
  ///
  /// # Returns
  ///
  /// `Ok(())` if the state was set successfully, `Err(StateError)` otherwise.
  ///
  /// # Errors
  ///
  /// Returns an error if:
  /// - The node is not stateful
  /// - The state type doesn't match
  /// - State access fails
  fn set_state(&self, state: Box<dyn std::any::Any + Send + Sync>) -> StateResult<()>;

  /// Resets the state of this node to its initial value.
  ///
  /// # Returns
  ///
  /// `Ok(())` if the state was reset successfully, `Err(StateError)` otherwise.
  fn reset_state(&self) -> StateResult<()>;

  /// Returns whether this node is stateful.
  ///
  /// # Returns
  ///
  /// `true` if this node is stateful, `false` otherwise.
  fn is_stateful(&self) -> bool;

  /// Returns whether this node has initialized state.
  ///
  /// # Returns
  ///
  /// `true` if the node is stateful and has initialized state, `false` otherwise.
  fn has_state(&self) -> bool;
}

/// Extension trait for TransformerNode to add stateful capabilities.
///
/// This trait is automatically implemented for `TransformerNode` that wraps
/// a `StatefulTransformer`, providing state access through the graph API.
impl<T, Inputs, Outputs> StatefulNode for TransformerNode<T, Inputs, Outputs>
where
  T: Transformer + StatefulTransformer + Send + Sync + 'static,
  T::Input: std::fmt::Debug + Clone + Send + Sync + serde::de::DeserializeOwned,
  T::Output: std::fmt::Debug + Clone + Send + Sync + serde::Serialize,
  Inputs: streamweave::port::PortList + Send + Sync + 'static,
  Outputs: streamweave::port::PortList + Send + Sync + 'static,
  (): crate::node::ValidateTransformerPorts<T, Inputs, Outputs>,
{
  fn get_state(&self) -> StateResult<Option<Box<dyn std::any::Any + Send + Sync>>> {
    let transformer = self.transformer();
    let result = if tokio::runtime::Handle::try_current().is_ok() {
      // We're in a runtime, spawn a new thread with a new runtime to avoid "runtime within runtime" error
      let transformer_clone = transformer.clone();
      std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
          .enable_all()
          .build()
          .unwrap()
          .block_on(async {
            let guard = transformer_clone.lock().await;
            guard.state()
          })
      })
      .join()
      .unwrap()
    } else {
      // Not in a runtime, create one and use block_on
      tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
          let guard = transformer.lock().await;
          guard.state()
        })
    };
    match result {
      Ok(Some(state)) => {
        // Box the state as Any for type erasure
        Ok(Some(Box::new(state)))
      }
      Ok(None) => Ok(None),
      Err(e) => Err(e),
    }
  }

  fn set_state(&self, state: Box<dyn std::any::Any + Send + Sync>) -> StateResult<()> {
    // Try to downcast to the transformer's state type
    // We need to extract the value from the box
    if let Ok(typed_state) = state.downcast::<T::State>() {
      let transformer = self.transformer();
      if tokio::runtime::Handle::try_current().is_ok() {
        // We're in a runtime, spawn a new thread with a new runtime to avoid "runtime within runtime" error
        let transformer_clone = transformer.clone();
        std::thread::spawn(move || {
          tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
              let guard = transformer_clone.lock().await;
              guard.set_state(*typed_state)
            })
        })
        .join()
        .unwrap()
      } else {
        // Not in a runtime, create one and use block_on
        tokio::runtime::Builder::new_current_thread()
          .enable_all()
          .build()
          .unwrap()
          .block_on(async {
            let guard = transformer.lock().await;
            guard.set_state(*typed_state)
          })
      }
    } else {
      Err(StateError::UpdateFailed(format!(
        "State type mismatch: expected {}, got different type",
        std::any::type_name::<T::State>()
      )))
    }
  }

  fn reset_state(&self) -> StateResult<()> {
    let transformer = self.transformer();
    if tokio::runtime::Handle::try_current().is_ok() {
      // We're in a runtime, spawn a new thread with a new runtime to avoid "runtime within runtime" error
      let transformer_clone = transformer.clone();
      std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
          .enable_all()
          .build()
          .unwrap()
          .block_on(async {
            let guard = transformer_clone.lock().await;
            guard.reset_state()
          })
      })
      .join()
      .unwrap()
    } else {
      // Not in a runtime, create one and use block_on
      tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
          let guard = transformer.lock().await;
          guard.reset_state()
        })
    }
  }

  fn is_stateful(&self) -> bool {
    true
  }

  fn has_state(&self) -> bool {
    let transformer = self.transformer();
    if tokio::runtime::Handle::try_current().is_ok() {
      // We're in a runtime, spawn a new thread with a new runtime to avoid "runtime within runtime" error
      let transformer_clone = transformer.clone();
      std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
          .enable_all()
          .build()
          .unwrap()
          .block_on(async {
            let guard = transformer_clone.lock().await;
            guard.has_state()
          })
      })
      .join()
      .unwrap()
    } else {
      // Not in a runtime, create one and use block_on
      tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
          let guard = transformer.lock().await;
          guard.has_state()
        })
    }
  }
}

/// Helper function to check if a node is stateful.
///
/// # Arguments
///
/// * `node` - The node to check
///
/// # Returns
///
/// `true` if the node implements `StatefulNode` and is stateful, `false` otherwise.
pub fn is_stateful_node(node: &dyn NodeTrait) -> bool {
  // Use the as_stateful() method from NodeTrait
  node.as_stateful().is_some()
}

/// Helper function to get state from a stateful node.
///
/// # Arguments
///
/// * `node` - The node to get state from
///
/// # Returns
///
/// `Some(state)` if the node is stateful and has state, `None` otherwise.
///
/// # Errors
///
/// Returns an error if state access fails.
pub fn get_node_state(
  node: &dyn NodeTrait,
) -> StateResult<Option<Box<dyn std::any::Any + Send + Sync>>> {
  // Try using as_stateful() first, then fall back to Any downcasting
  if let Some(stateful) = node.as_stateful() {
    stateful.get_state()
  } else {
    // Fallback: use Any trait downcasting to check if node is StatefulNode
    // This is safe because we're checking the type_id first
    // We can't directly downcast to a trait object, so we need to use a different approach
    // For now, return error indicating node is not stateful
    Err(StateError::NotInitialized)
  }
}

/// Helper function to set state on a stateful node.
///
/// # Arguments
///
/// * `node` - The node to set state on
/// * `state` - The new state value (must match the node's state type)
///
/// # Returns
///
/// `Ok(())` if the state was set successfully, `Err(StateError)` otherwise.
///
/// # Errors
///
/// Returns an error if:
/// - The node is not stateful
/// - The state type doesn't match
/// - State access fails
pub fn set_node_state(
  node: &dyn NodeTrait,
  state: Box<dyn std::any::Any + Send + Sync>,
) -> StateResult<()> {
  // Use the as_stateful() method from NodeTrait
  if let Some(stateful) = node.as_stateful() {
    stateful.set_state(state)
  } else {
    Err(StateError::NotInitialized)
  }
}

/// Helper function to reset state on a stateful node.
///
/// # Arguments
///
/// * `node` - The node to reset state on
///
/// # Returns
///
/// `Ok(())` if the state was reset successfully, `Err(StateError)` otherwise.
///
/// # Errors
///
/// Returns an error if:
/// - The node is not stateful
/// - State access fails
pub fn reset_node_state(node: &dyn NodeTrait) -> StateResult<()> {
  // Use the as_stateful() method from NodeTrait
  if let Some(stateful) = node.as_stateful() {
    stateful.reset_state()
  } else {
    Err(StateError::NotInitialized)
  }
}
