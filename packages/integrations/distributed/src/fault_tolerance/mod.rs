//! Fault tolerance mechanisms for distributed processing.
//!
//! Provides heartbeat monitoring, failure detection, work redistribution,
//! and state recovery from checkpoints.

pub mod checkpoint;
pub mod failure_detector;
pub mod recovery;

pub use checkpoint::{Checkpoint, CheckpointError, CheckpointStore};
pub use failure_detector::{FailureDetector, FailureDetectorConfig, FailureEvent};
pub use recovery::{RecoveryManager, RecoveryStrategy};
