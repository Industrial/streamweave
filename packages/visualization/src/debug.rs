//! # Pipeline Debug Mode
//!
//! This module provides debugging capabilities for StreamWeave pipelines,
//! including breakpoints, step-through execution, and data inspection.
//!
//! ## Example
//!
//! ```rust
//! use streamweave::visualization::debug::{Debugger, Breakpoint};
//!
//! let mut debugger = Debugger::new();
//! debugger.add_breakpoint(Breakpoint::at_node("transformer_1".to_string()));
//! // ... configure pipeline with debugger ...
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A breakpoint in the pipeline for debugging.
///
/// Breakpoints pause pipeline execution at specific nodes,
/// allowing inspection of data at that stage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Breakpoint {
  /// Node ID where the breakpoint is set
  pub node_id: String,
  /// Optional condition that must be true for breakpoint to trigger
  pub condition: Option<String>,
  /// Whether the breakpoint is enabled
  pub enabled: bool,
}

impl Breakpoint {
  /// Creates a new breakpoint at the specified node.
  ///
  /// # Arguments
  ///
  /// * `node_id` - The ID of the node where execution should pause
  ///
  /// # Returns
  ///
  /// A new enabled breakpoint.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Breakpoint;
  ///
  /// let breakpoint = Breakpoint::at_node("transformer_1".to_string());
  /// ```
  #[must_use]
  pub fn at_node(node_id: String) -> Self {
    Self {
      node_id,
      condition: None,
      enabled: true,
    }
  }

  /// Creates a new breakpoint with a condition.
  ///
  /// # Arguments
  ///
  /// * `node_id` - The ID of the node where execution should pause
  /// * `condition` - A condition expression (e.g., "item.value > 100")
  ///
  /// # Returns
  ///
  /// A new enabled breakpoint with the condition.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Breakpoint;
  ///
  /// let breakpoint = Breakpoint::with_condition(
  ///     "transformer_1".to_string(),
  ///     "item.value > 100".to_string(),
  /// );
  /// ```
  #[must_use]
  pub fn with_condition(node_id: String, condition: String) -> Self {
    Self {
      node_id,
      condition: Some(condition),
      enabled: true,
    }
  }

  /// Disables the breakpoint.
  ///
  /// Disabled breakpoints will not pause execution.
  pub fn disable(&mut self) {
    self.enabled = false;
  }

  /// Enables the breakpoint.
  ///
  /// Enabled breakpoints will pause execution when reached.
  pub fn enable(&mut self) {
    self.enabled = true;
  }
}

/// State of the debugger during pipeline execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugState {
  /// Running normally (no breakpoint hit)
  Running,
  /// Paused at a breakpoint
  Paused {
    /// Node ID where execution is paused
    node_id: String,
    /// Data item at the pause point (if available)
    data_snapshot: Option<String>,
  },
  /// Stepping through execution
  Stepping,
  /// Execution completed
  Completed,
}

/// Snapshot of data at a breakpoint for inspection.
///
/// This allows examining the data item that triggered the breakpoint
/// without consuming or modifying it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSnapshot {
  /// Node ID where the snapshot was taken
  pub node_id: String,
  /// Serialized representation of the data item
  pub data: String,
  /// Metadata about the snapshot
  pub metadata: HashMap<String, String>,
}

impl DataSnapshot {
  /// Creates a new data snapshot.
  ///
  /// # Arguments
  ///
  /// * `node_id` - The node where the snapshot was taken
  /// * `data` - Serialized representation of the data
  ///
  /// # Returns
  ///
  /// A new data snapshot.
  #[must_use]
  pub fn new(node_id: String, data: String) -> Self {
    Self {
      node_id,
      data,
      metadata: HashMap::new(),
    }
  }
}

/// Debugger for controlling pipeline execution and inspecting data.
///
/// The debugger manages breakpoints, execution state, and data inspection
/// for step-through debugging of pipelines.
///
/// # Example
///
/// ```rust
/// use streamweave::visualization::debug::{Debugger, Breakpoint};
///
/// let mut debugger = Debugger::new();
/// debugger.add_breakpoint(Breakpoint::at_node("transformer_1".to_string()));
/// // ... use debugger with pipeline ...
/// ```
#[derive(Debug, Clone)]
pub struct Debugger {
  /// Breakpoints that should pause execution
  pub breakpoints: Arc<RwLock<Vec<Breakpoint>>>,
  /// Current debug state
  pub state: Arc<RwLock<DebugState>>,
  /// Data snapshots captured at breakpoints
  pub snapshots: Arc<RwLock<Vec<DataSnapshot>>>,
  /// History of execution steps
  pub history: Arc<RwLock<Vec<String>>>,
}

impl Debugger {
  /// Creates a new debugger instance.
  ///
  /// # Returns
  ///
  /// A new debugger in the Running state with no breakpoints.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Debugger;
  ///
  /// let debugger = Debugger::new();
  /// ```
  #[must_use]
  pub fn new() -> Self {
    Self {
      breakpoints: Arc::new(RwLock::new(Vec::new())),
      state: Arc::new(RwLock::new(DebugState::Running)),
      snapshots: Arc::new(RwLock::new(Vec::new())),
      history: Arc::new(RwLock::new(Vec::new())),
    }
  }

  /// Adds a breakpoint to the debugger.
  ///
  /// # Arguments
  ///
  /// * `breakpoint` - The breakpoint to add
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::{Debugger, Breakpoint};
  ///
  /// # async fn example() {
  /// let mut debugger = Debugger::new();
  /// debugger.add_breakpoint(Breakpoint::at_node("transformer_1".to_string())).await;
  /// # }
  /// ```
  pub async fn add_breakpoint(&mut self, breakpoint: Breakpoint) {
    let mut breakpoints = self.breakpoints.write().await;
    breakpoints.push(breakpoint);
  }

  /// Removes a breakpoint at the specified node.
  ///
  /// # Arguments
  ///
  /// * `node_id` - The node ID to remove breakpoints from
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Debugger;
  ///
  /// # async fn example() {
  /// let mut debugger = Debugger::new();
  /// debugger.remove_breakpoint("transformer_1".to_string()).await;
  /// # }
  /// ```
  pub async fn remove_breakpoint(&mut self, node_id: String) {
    let mut breakpoints = self.breakpoints.write().await;
    breakpoints.retain(|bp| bp.node_id != node_id);
  }

  /// Checks if execution should pause at the given node.
  ///
  /// This is called by the pipeline execution to determine
  /// if a breakpoint has been hit.
  ///
  /// # Arguments
  ///
  /// * `node_id` - The node ID to check
  /// * `data_snapshot` - Optional serialized data for inspection
  ///
  /// # Returns
  ///
  /// `true` if execution should pause, `false` otherwise.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Debugger;
  ///
  /// # async fn example() {
  /// let debugger = Debugger::new();
  /// let should_pause = debugger.should_pause("transformer_1".to_string(), None).await;
  /// # }
  /// ```
  pub async fn should_pause(&self, node_id: String, data_snapshot: Option<String>) -> bool {
    let state = self.state.read().await;

    // If already paused or stepping, check state
    match *state {
      DebugState::Paused { .. } => return true,
      DebugState::Stepping => {
        // Step mode - pause at next node
        return true;
      }
      DebugState::Running | DebugState::Completed => {}
    }

    drop(state);

    // Check if there's a breakpoint at this node
    let breakpoints = self.breakpoints.read().await;
    for breakpoint in breakpoints.iter() {
      if breakpoint.enabled && breakpoint.node_id == node_id {
        // Breakpoint hit - pause execution
        let mut state = self.state.write().await;
        *state = DebugState::Paused {
          node_id: node_id.clone(),
          data_snapshot: data_snapshot.clone(),
        };

        // Save snapshot
        if let Some(ref data) = data_snapshot {
          let mut snapshots = self.snapshots.write().await;
          snapshots.push(DataSnapshot::new(node_id.clone(), data.clone()));
        }

        return true;
      }
    }

    false
  }

  /// Resumes execution from a paused state.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Debugger;
  ///
  /// # async fn example() {
  /// let debugger = Debugger::new();
  /// debugger.resume().await;
  /// # }
  /// ```
  pub async fn resume(&self) {
    let mut state = self.state.write().await;
    if matches!(*state, DebugState::Paused { .. }) {
      *state = DebugState::Running;
    }
  }

  /// Steps execution forward one item.
  ///
  /// After stepping, execution will pause at the next node.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Debugger;
  ///
  /// # async fn example() {
  /// let debugger = Debugger::new();
  /// debugger.step().await;
  /// # }
  /// ```
  pub async fn step(&self) {
    let mut state = self.state.write().await;
    *state = DebugState::Stepping;
  }

  /// Steps execution backward one step in history.
  ///
  /// This allows inspecting previous execution states.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Debugger;
  ///
  /// # async fn example() {
  /// let debugger = Debugger::new();
  /// debugger.step_back().await;
  /// # }
  /// ```
  pub async fn step_back(&self) {
    // Implementation would restore previous state from history
    // For now, this is a placeholder
    let _history = self.history.read().await;
  }

  /// Gets the current debug state.
  ///
  /// # Returns
  ///
  /// A clone of the current debug state.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Debugger;
  ///
  /// # async fn example() {
  /// let debugger = Debugger::new();
  /// let state = debugger.get_state().await;
  /// # }
  /// ```
  pub async fn get_state(&self) -> DebugState {
    self.state.read().await.clone()
  }

  /// Gets all data snapshots captured during debugging.
  ///
  /// # Returns
  ///
  /// A vector of all captured snapshots.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Debugger;
  ///
  /// # async fn example() {
  /// let debugger = Debugger::new();
  /// let snapshots = debugger.get_snapshots().await;
  /// # }
  /// ```
  pub async fn get_snapshots(&self) -> Vec<DataSnapshot> {
    self.snapshots.read().await.clone()
  }

  /// Clears all breakpoints.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Debugger;
  ///
  /// # async fn example() {
  /// let mut debugger = Debugger::new();
  /// debugger.clear_breakpoints().await;
  /// # }
  /// ```
  pub async fn clear_breakpoints(&mut self) {
    let mut breakpoints = self.breakpoints.write().await;
    breakpoints.clear();
  }

  /// Resets the debugger to initial state.
  ///
  /// Clears all breakpoints, snapshots, and history.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::visualization::debug::Debugger;
  ///
  /// # async fn example() {
  /// let mut debugger = Debugger::new();
  /// debugger.reset().await;
  /// # }
  /// ```
  pub async fn reset(&mut self) {
    let mut state = self.state.write().await;
    *state = DebugState::Running;
    drop(state);

    self.clear_breakpoints().await;

    let mut snapshots = self.snapshots.write().await;
    snapshots.clear();
    drop(snapshots);

    let mut history = self.history.write().await;
    history.clear();
  }
}

impl Default for Debugger {
  fn default() -> Self {
    Self::new()
  }
}
