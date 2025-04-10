//! Effect system implementation.
//!
//! This module provides a comprehensive effect system for Rust, including:
//! - Core effect types and traits
//! - Error handling
//! - Resource management
//! - Concurrency support
//! - Effect handlers
//! - Utility functions and types

pub mod concurrent;
pub mod core;
pub mod error;
pub mod handlers;
pub mod resource;
pub mod utils;
