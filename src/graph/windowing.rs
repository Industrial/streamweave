//! # Graph Windowing Operations
//!
//! This module provides windowing operations that work with graph topology.
//! It integrates the existing windowing infrastructure with the graph API,
//! enabling time-based and count-based windows in graph processing.
//!
//! # Overview
//!
//! Windowing in graphs allows you to:
//! - Apply windows to specific nodes in the graph
//! - Create windows that span multiple nodes
//! - Configure window behavior per node or globally
//! - Handle late data in graph contexts
//!
//! # Window Types
//!
//! - **Time-based windows**: Group items by time intervals
//! - **Count-based windows**: Group items by count thresholds
//! - **Session windows**: Group by activity gaps
//!
//! # Example
//!
//! ```rust,no_run
//! use streamweave::graph::{GraphBuilder, TransformerNode};
//! use streamweave::transformers::window::WindowTransformer;
//!
//! // Create a graph with a windowing node
//! let graph = GraphBuilder::new()
//!     .node(TransformerNode::new(
//!         "window".to_string(),
//!         WindowTransformer::new(10), // 10-item window
//!     ))
//!     .unwrap()
//!     .build()
//!     .unwrap();
//! ```

use crate::graph::node::TransformerNode;
use crate::graph::traits::NodeTrait;
use crate::transformers::window::window_transformer::WindowTransformer;
use crate::window::{WindowError, WindowResult};
use std::time::Duration;

/// Configuration for windowing operations in graphs.
///
/// This struct provides configuration options for applying windows
/// to nodes in a graph, including window type, size, and behavior.
#[derive(Debug, Clone)]
pub struct GraphWindowConfig {
  /// Window size (for count-based windows) or duration (for time-based windows)
  pub size: WindowSize,
  /// Window type (tumbling, sliding, session)
  pub window_type: WindowType,
  /// Whether to emit partial windows
  pub emit_partial: bool,
  /// Late data policy
  pub late_data_policy: crate::window::LateDataPolicy,
}

/// Window size configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowSize {
  /// Count-based window (number of items)
  Count(usize),
  /// Time-based window (duration)
  Time(Duration),
}

/// Window type configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
  /// Tumbling window (fixed-size, non-overlapping)
  Tumbling,
  /// Sliding window (fixed-size, overlapping)
  Sliding {
    /// Slide interval (for sliding windows)
    slide: WindowSize,
  },
  /// Session window (gap-based)
  Session {
    /// Gap duration for session windows
    gap: Duration,
  },
}

impl Default for GraphWindowConfig {
  fn default() -> Self {
    Self {
      size: WindowSize::Count(10),
      window_type: WindowType::Tumbling,
      emit_partial: false,
      late_data_policy: crate::window::LateDataPolicy::Drop,
    }
  }
}

impl GraphWindowConfig {
  /// Creates a new count-based tumbling window configuration.
  ///
  /// # Arguments
  ///
  /// * `size` - The number of items per window
  ///
  /// # Returns
  ///
  /// A new `GraphWindowConfig` with count-based tumbling windows.
  pub fn count_tumbling(size: usize) -> Self {
    Self {
      size: WindowSize::Count(size),
      window_type: WindowType::Tumbling,
      emit_partial: false,
      late_data_policy: crate::window::LateDataPolicy::Drop,
    }
  }

  /// Creates a new time-based tumbling window configuration.
  ///
  /// # Arguments
  ///
  /// * `duration` - The duration of each window
  ///
  /// # Returns
  ///
  /// A new `GraphWindowConfig` with time-based tumbling windows.
  pub fn time_tumbling(duration: Duration) -> Self {
    Self {
      size: WindowSize::Time(duration),
      window_type: WindowType::Tumbling,
      emit_partial: false,
      late_data_policy: crate::window::LateDataPolicy::Drop,
    }
  }

  /// Creates a new count-based sliding window configuration.
  ///
  /// # Arguments
  ///
  /// * `size` - The number of items per window
  /// * `slide` - The number of items to slide by
  ///
  /// # Returns
  ///
  /// A new `GraphWindowConfig` with count-based sliding windows.
  pub fn count_sliding(size: usize, slide: usize) -> Self {
    Self {
      size: WindowSize::Count(size),
      window_type: WindowType::Sliding {
        slide: WindowSize::Count(slide),
      },
      emit_partial: false,
      late_data_policy: crate::window::LateDataPolicy::Drop,
    }
  }

  /// Creates a new time-based sliding window configuration.
  ///
  /// # Arguments
  ///
  /// * `size` - The duration of each window
  /// * `slide` - The duration to slide by
  ///
  /// # Returns
  ///
  /// A new `GraphWindowConfig` with time-based sliding windows.
  pub fn time_sliding(size: Duration, slide: Duration) -> Self {
    Self {
      size: WindowSize::Time(size),
      window_type: WindowType::Sliding {
        slide: WindowSize::Time(slide),
      },
      emit_partial: false,
      late_data_policy: crate::window::LateDataPolicy::Drop,
    }
  }

  /// Sets whether to emit partial windows.
  ///
  /// # Arguments
  ///
  /// * `emit` - `true` to emit partial windows, `false` otherwise
  ///
  /// # Returns
  ///
  /// Self for method chaining.
  pub fn with_emit_partial(mut self, emit: bool) -> Self {
    self.emit_partial = emit;
    self
  }

  /// Sets the late data policy.
  ///
  /// # Arguments
  ///
  /// * `policy` - The late data policy to use
  ///
  /// # Returns
  ///
  /// Self for method chaining.
  pub fn with_late_data_policy(mut self, policy: crate::window::LateDataPolicy) -> Self {
    self.late_data_policy = policy;
    self
  }
}

/// Helper function to create a window transformer node from a graph window configuration.
///
/// # Arguments
///
/// * `name` - The name for the window node
/// * `config` - The window configuration
///
/// # Returns
///
/// A `TransformerNode` wrapping a `WindowTransformer` configured according to the config.
///
/// # Note
///
/// Currently, this only supports count-based windows as `WindowTransformer` is count-based.
/// Time-based windows will require additional implementation.
type WindowNode<T> = TransformerNode<WindowTransformer<T>, (T,), (Vec<T>,)>;

/// Creates a window node for use in graph processing.
///
/// This function creates a transformer node that applies windowing operations
/// to stream items according to the provided configuration.
///
/// # Arguments
///
/// * `name` - The name for the window node
/// * `config` - Window configuration specifying size and type
///
/// # Returns
///
/// A `WindowResult` containing the created window node on success.
pub fn create_window_node<T>(name: String, config: GraphWindowConfig) -> WindowResult<WindowNode<T>>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  match config.size {
    WindowSize::Count(size) => {
      let transformer = WindowTransformer::new(size);
      Ok(TransformerNode::new(name, transformer))
    }
    WindowSize::Time(_) => {
      // Time-based windows require additional implementation
      // For now, return an error
      Err(WindowError::InvalidConfig(
        "Time-based windows not yet implemented for graph API".to_string(),
      ))
    }
  }
}

/// Extension trait for nodes to add windowing capabilities.
///
/// This trait allows querying whether a node performs windowing operations.
pub trait WindowedNode: NodeTrait {
  /// Returns the window configuration for this node, if it is a windowing node.
  ///
  /// # Returns
  ///
  /// `Some(config)` if this node performs windowing, `None` otherwise.
  fn window_config(&self) -> Option<GraphWindowConfig>;

  /// Returns whether this node performs windowing operations.
  ///
  /// # Returns
  ///
  /// `true` if this node is a windowing node, `false` otherwise.
  fn is_windowed(&self) -> bool;
}

/// Implementation of `WindowedNode` for `TransformerNode` wrapping `WindowTransformer`.
impl<T, Inputs, Outputs> WindowedNode for TransformerNode<WindowTransformer<T>, Inputs, Outputs>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  Inputs: crate::graph::port::PortList + Send + Sync,
  Outputs: crate::graph::port::PortList + Send + Sync,
  (): crate::graph::node::ValidateTransformerPorts<WindowTransformer<T>, Inputs, Outputs>,
{
  fn window_config(&self) -> Option<GraphWindowConfig> {
    // Extract window size from the transformer
    let size = self.transformer().size;
    Some(GraphWindowConfig {
      size: WindowSize::Count(size),
      window_type: WindowType::Tumbling, // WindowTransformer is tumbling by default
      emit_partial: true,                // WindowTransformer emits partial windows
      late_data_policy: crate::window::LateDataPolicy::Drop,
    })
  }

  fn is_windowed(&self) -> bool {
    true
  }
}

/// Helper function to check if a node performs windowing operations.
///
/// # Arguments
///
/// * `node` - The node to check
///
/// # Returns
///
/// `true` if the node is a windowing node, `false` otherwise.
pub fn is_windowed_node(_node: &dyn NodeTrait) -> bool {
  // This would require dynamic dispatch and downcasting
  // For now, return false as a placeholder
  false
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_graph_window_config() {
    let config = GraphWindowConfig::count_tumbling(10);
    assert_eq!(config.size, WindowSize::Count(10));
    assert_eq!(config.window_type, WindowType::Tumbling);

    let config = GraphWindowConfig::time_tumbling(Duration::from_secs(5));
    assert_eq!(config.size, WindowSize::Time(Duration::from_secs(5)));

    let config = GraphWindowConfig::count_sliding(10, 5);
    match config.window_type {
      WindowType::Sliding { slide } => {
        assert_eq!(slide, WindowSize::Count(5));
      }
      _ => panic!("Expected sliding window"),
    }
  }

  #[test]
  fn test_create_window_node() {
    let config = GraphWindowConfig::count_tumbling(10);
    let node = create_window_node::<i32>("window".to_string(), config).unwrap();

    assert_eq!(node.name(), "window");
    assert!(node.is_windowed());

    let window_config = node.window_config().unwrap();
    assert_eq!(window_config.size, WindowSize::Count(10));
  }

  #[test]
  fn test_window_node_trait() {
    let transformer = WindowTransformer::<i32>::new(10);
    let node: TransformerNode<WindowTransformer<i32>, (i32,), (Vec<i32>,)> =
      TransformerNode::new("window".to_string(), transformer);

    assert!(node.is_windowed());
    let config = node.window_config().unwrap();
    assert_eq!(config.size, WindowSize::Count(10));
  }
}
