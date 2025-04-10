//! Concurrency support for the effect system.
//!
//! This module provides types and traits for concurrent execution of effects,
//! including structured concurrency, supervision, and cancellation.

pub mod cancellation;
pub mod structured;
pub mod supervision;

pub use cancellation::{Cancellable, CancellationToken};
pub use structured::{parallel, sequence};
pub use supervision::{Supervisor, SupervisorStrategy};
