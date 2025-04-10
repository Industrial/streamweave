//! Resource management module for the effect system.
//!
//! This module provides types and traits for managing resources in the effect
//! system. It includes the bracket pattern for resource acquisition and release,
//! resource scoping, and resource guards.

pub mod bracket;
pub mod guard;
pub mod scope;
