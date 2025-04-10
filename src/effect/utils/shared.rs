//! Shared state utilities for the effect system.
//!
//! This module provides types and utilities for managing shared state in the
//! effect system.

use std::sync::Arc;
use tokio::sync::Mutex;

/// A type alias for shared state that can be safely accessed from multiple threads.
pub type Shared<T> = Arc<Mutex<T>>;
