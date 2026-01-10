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
//! transformer fields.
//!
//! ## `Message<T>` Support
//!
//! When this module is re-enabled, stateful nodes will work with `Message<T>` types.
//! All data flowing through stateful nodes is wrapped in `Message<T>`, preserving
//! message IDs and metadata throughout stateful operations. Stateful transformers
//! receive unwrapped payloads (`T`) and produce outputs that are automatically
//! wrapped in `Message<T>` before being sent to downstream nodes.
//!
//! ### Checkpointing with `Message<T>`
//!
//! When checkpointing state, the stateful node implementation should preserve
//! message metadata when possible. Checkpointed state should include:
//! - Message IDs for the last processed messages (for recovery)
//! - Message metadata relevant to state transitions
//! - State snapshots that can be restored with message context
//!
//! This ensures that when state is restored from a checkpoint, the message
//! traceability is maintained across stateful operations.

// Note: This module is temporarily disabled due to circular dependency issues.
// The stateful package depends on streamweave, and streamweave would depend on stateful.
// This will be resolved in a future refactoring by either:
// 1. Moving stateful support to the stateful package itself
// 2. Breaking the circular dependency by restructuring the packages
// 3. Making stateful support optional at a different level

#![allow(unexpected_cfgs)]

// TODO: Re-enable when circular dependency is resolved
// The original implementation is preserved in packages/graph/src/stateful.rs
