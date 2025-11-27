//! Debounce transformer module.
//!
//! This module provides functionality for debouncing items in a stream,
//! ensuring that only the last item within a time window is emitted.

/// The debounce transformer implementation.
pub mod debounce_transformer;
/// Input types for the debounce transformer.
pub mod input;
/// Output types for the debounce transformer.
pub mod output;
/// Transformer trait implementation for debounce.
pub mod transformer;
