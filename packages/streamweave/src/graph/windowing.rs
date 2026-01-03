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
//! ## Message<T> Support
//!
//! When this module is re-enabled, windowing operations will work with `Message<T>` types.
//! All data flowing through windowed nodes is wrapped in `Message<T>`, preserving
//! message IDs and metadata throughout windowing operations. Windowed transformers
//! receive unwrapped payloads (`T`) and produce windowed outputs that are automatically
//! wrapped in `Message<T>` before being sent to downstream nodes.
//!
//! ### Message Metadata Preservation
//!
//! When windows are emitted, the windowing implementation should:
//! - Preserve message IDs from the original messages in the window
//! - Aggregate or merge message metadata appropriately (e.g., timestamps, headers)
//! - Maintain traceability from windowed outputs back to source messages
//! - Handle late data with appropriate message metadata updates
//!
//! This ensures that windowed outputs maintain end-to-end traceability and can be
//! correlated back to their source messages for debugging and monitoring.

// Note: This module is temporarily disabled due to circular dependency issues.
// The window package depends on streamweave, and streamweave would depend on window.
// This will be resolved in a future refactoring by either:
// 1. Moving windowing support to the window package itself
// 2. Breaking the circular dependency by restructuring the packages
// 3. Making windowing support optional at a different level

#![cfg(feature = "windowing")]

// TODO: Re-enable when circular dependency is resolved
// The original implementation is preserved in packages/graph/src/windowing.rs
