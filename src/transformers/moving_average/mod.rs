//! Moving average stateful transformer.
//!
//! This module provides a stateful transformer that calculates a moving average
//! over a configurable window of recent items.

mod moving_average_transformer;
mod transformer;

pub use moving_average_transformer::MovingAverageTransformer;
