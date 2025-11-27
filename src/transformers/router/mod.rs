//! Router transformer module.
//!
//! This module provides the `RouterTransformer` which routes items to different
//! output streams based on a routing function.

/// Input type definitions for the router transformer.
pub mod input;
/// Output type definitions for the router transformer.
pub mod output;
/// The router transformer implementation.
pub mod router_transformer;
/// Transformer trait implementation.
pub mod transformer;
