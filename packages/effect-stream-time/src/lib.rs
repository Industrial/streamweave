//! Time-based streams for the Effect ecosystem.
//!
//! This package provides streams that produce values based on time. It includes
//! both interval-based streams (for periodic values) and timer-based streams
//! (for one-time values).

pub mod error;
pub mod time;

pub use error::TimeStreamError;
pub use time::{TestTimeSource, TimeStream, TimeStreamSource};

/// Re-exports from effect-stream for convenience
pub use effect_stream::{EffectResult, EffectStream, EffectStreamSource};
