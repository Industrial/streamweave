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
//! ## Architecture
//!
//! Windowed nodes access transformers through `Arc<tokio::sync::Mutex<T>>` to
//! read window configuration. The `window_config()` method uses runtime detection
//! to safely lock the mutex and access transformer fields.
//!
//! # Example
//!
//! ```rust,no_run
//! use streamweave::graph::{GraphBuilder, TransformerNode};
//! use streamweave_transformers::WindowTransformer;
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

use crate::node::TransformerNode;
use crate::traits::NodeTrait;
use std::time::Duration;
use streamweave_window::WindowResult;
use streamweave_window::{TimeWindowTransformer, WindowTransformer};

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
  pub late_data_policy: streamweave_window::LateDataPolicy,
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
      late_data_policy: streamweave_window::LateDataPolicy::Drop,
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
      late_data_policy: streamweave_window::LateDataPolicy::Drop,
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
      late_data_policy: streamweave_window::LateDataPolicy::Drop,
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
      late_data_policy: streamweave_window::LateDataPolicy::Drop,
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
      late_data_policy: streamweave_window::LateDataPolicy::Drop,
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
  pub fn with_late_data_policy(mut self, policy: streamweave_window::LateDataPolicy) -> Self {
    self.late_data_policy = policy;
    self
  }
}

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
/// A `WindowResult` containing the created window node as a boxed `NodeTrait` on success.
pub fn create_window_node<T>(
  name: String,
  config: GraphWindowConfig,
) -> WindowResult<Box<dyn NodeTrait>>
where
  T: std::fmt::Debug
    + Clone
    + Send
    + Sync
    + serde::Serialize
    + for<'de> serde::Deserialize<'de>
    + 'static,
{
  match config.size {
    WindowSize::Count(size) => {
      let transformer = WindowTransformer::new(size);
      let node: TransformerNode<WindowTransformer<T>, (T,), (Vec<T>,)> = TransformerNode::new(
        name,
        transformer,
        vec!["in".to_string()],
        vec!["out".to_string()],
      );
      Ok(Box::new(node))
    }
    WindowSize::Time(duration) => {
      let transformer = TimeWindowTransformer::new(duration);
      let node: TransformerNode<TimeWindowTransformer<T>, (T,), (Vec<T>,)> = TransformerNode::new(
        name,
        transformer,
        vec!["in".to_string()],
        vec!["out".to_string()],
      );
      Ok(Box::new(node))
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
  T: std::fmt::Debug
    + Clone
    + Send
    + Sync
    + serde::Serialize
    + for<'de> serde::Deserialize<'de>
    + 'static,
  Inputs: streamweave::port::PortList + Send + Sync + 'static,
  Outputs: streamweave::port::PortList + Send + Sync + 'static,
  (): crate::node::ValidateTransformerPorts<WindowTransformer<T>, Inputs, Outputs>,
{
  fn window_config(&self) -> Option<GraphWindowConfig> {
    // Extract window size from the transformer
    let transformer = self.transformer();
    let size = if tokio::runtime::Handle::try_current().is_ok() {
      // We're in a runtime, spawn a new thread with a new runtime to avoid "runtime within runtime" error
      let transformer_clone = transformer.clone();
      std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
          .enable_all()
          .build()
          .unwrap()
          .block_on(async {
            let guard = transformer_clone.lock().await;
            guard.size
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
          guard.size
        })
    };
    Some(GraphWindowConfig {
      size: WindowSize::Count(size),
      window_type: WindowType::Tumbling, // WindowTransformer is tumbling by default
      emit_partial: true,                // WindowTransformer emits partial windows
      late_data_policy: streamweave_window::LateDataPolicy::Drop,
    })
  }

  fn is_windowed(&self) -> bool {
    true
  }
}

/// Implementation of `WindowedNode` for `TransformerNode` wrapping `TimeWindowTransformer`.
impl<T, Inputs, Outputs> WindowedNode for TransformerNode<TimeWindowTransformer<T>, Inputs, Outputs>
where
  T: std::fmt::Debug
    + Clone
    + Send
    + Sync
    + serde::Serialize
    + for<'de> serde::Deserialize<'de>
    + 'static,
  Inputs: streamweave::port::PortList + Send + Sync + 'static,
  Outputs: streamweave::port::PortList + Send + Sync + 'static,
  (): crate::node::ValidateTransformerPorts<TimeWindowTransformer<T>, Inputs, Outputs>,
{
  fn window_config(&self) -> Option<GraphWindowConfig> {
    // Extract window duration from the transformer
    let transformer = self.transformer();
    let duration = if tokio::runtime::Handle::try_current().is_ok() {
      // We're in a runtime, spawn a new thread with a new runtime to avoid "runtime within runtime" error
      let transformer_clone = transformer.clone();
      std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
          .enable_all()
          .build()
          .unwrap()
          .block_on(async {
            let guard = transformer_clone.lock().await;
            guard.duration
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
          guard.duration
        })
    };
    Some(GraphWindowConfig {
      size: WindowSize::Time(duration),
      window_type: WindowType::Tumbling, // TimeWindowTransformer is tumbling by default
      emit_partial: true,                // TimeWindowTransformer emits partial windows
      late_data_policy: streamweave_window::LateDataPolicy::Drop,
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
pub fn is_windowed_node(node: &dyn NodeTrait) -> bool {
  // Use the as_windowed() method from NodeTrait
  node.as_windowed().is_some()
}

/// Helper function to get window configuration from a windowed node.
///
/// # Arguments
///
/// * `node` - The node to get window config from
///
/// # Returns
///
/// `Some(config)` if the node is windowed, `None` otherwise.
pub fn window_config(node: &dyn NodeTrait) -> Option<GraphWindowConfig> {
  // Use the as_windowed() method from NodeTrait
  node
    .as_windowed()
    .and_then(|windowed| windowed.window_config())
}
