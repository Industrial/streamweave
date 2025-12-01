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

use crate::graph::node::TransformerNode;
use crate::graph::traits::NodeTrait;
use crate::stateful_transformer::{StateError, StatefulTransformer, StateResult};
use crate::transformer::Transformer;

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
  T: Transformer + StatefulTransformer + 'static,
  Inputs: crate::graph::port::PortList,
  Outputs: crate::graph::port::PortList,
  (): crate::graph::node::ValidateTransformerPorts<T, Inputs, Outputs>,
{
  fn get_state(&self) -> StateResult<Option<Box<dyn std::any::Any + Send + Sync>>> {
    match self.transformer().state() {
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
      self.transformer().set_state(*typed_state)
    } else {
      Err(StateError::UpdateFailed(format!(
        "State type mismatch: expected {}, got different type",
        std::any::type_name::<T::State>()
      )))
    }
  }

  fn reset_state(&self) -> StateResult<()> {
    self.transformer().reset_state()
  }

  fn is_stateful(&self) -> bool {
    true
  }

  fn has_state(&self) -> bool {
    self.transformer().has_state()
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
  // Use dynamic dispatch to check if node is stateful
  // This requires downcasting, which we'll handle in the graph execution
  false // Placeholder - will be implemented with proper type checking
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
  // This will need to use dynamic dispatch and downcasting
  // For now, return an error indicating the node is not stateful
  Err(StateError::NotInitialized)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::graph::node::TransformerNode;
  use crate::transformers::running_sum::RunningSumTransformer;

  #[test]
  fn test_stateful_node_trait() {
    let transformer = RunningSumTransformer::<i32>::new();
    let node: TransformerNode<RunningSumTransformer<i32>, (i32,), (i32,)> =
      TransformerNode::new("sum".to_string(), transformer);

    // Check that the node is stateful
    assert!(node.is_stateful());

    // Initially, state should not be initialized
    assert!(!node.has_state());

    // Get state should return None for uninitialized state
    let state = node.get_state().unwrap();
    assert!(state.is_none());

    // Reset should succeed even when not initialized
    assert!(node.reset_state().is_ok());
  }

  #[test]
  fn test_stateful_node_state_operations() {
    let transformer = RunningSumTransformer::<i32>::new();
    let node: TransformerNode<RunningSumTransformer<i32>, (i32,), (i32,)> =
      TransformerNode::new("sum".to_string(), transformer);

    // Set initial state
    let initial_state: Box<dyn std::any::Any + Send + Sync> = Box::new(10i32);
    assert!(node.set_state(initial_state).is_ok());

    // Now state should be initialized
    assert!(node.has_state());

    // Get state should return the set value
    let state = node.get_state().unwrap();
    assert!(state.is_some());

    // Reset state
    assert!(node.reset_state().is_ok());
    assert!(!node.has_state());
  }
}

