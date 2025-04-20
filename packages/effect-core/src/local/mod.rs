//! Local (non-thread-safe) trait implementations.
//!
//! This module contains trait-like implementations for types that
//! cannot implement the standard traits due to thread-safety requirements.

pub mod monoid;
pub mod semigroup;
