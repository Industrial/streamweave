//! Distributed stream processing architecture.
//!
//! This module defines the core architecture for distributed stream processing
//! across multiple nodes. It provides the foundation for coordinator/worker
//! patterns, communication protocols, state distribution, and fault tolerance.

pub mod coordinator;
pub mod fault_tolerance;
pub mod network;
pub mod partitioner;
pub mod protocol;
pub mod worker;

pub use coordinator::{Coordinator, CoordinatorConfig, CoordinatorError};
pub use partitioner::rebalance;
pub use partitioner::{
  CustomPartitioner, HashPartitioner, PartitionKey, PartitionStrategy, Partitioner,
  RangePartitioner, RoundRobinPartitioner,
};
pub use protocol::{Message, MessageType, ProtocolError, ProtocolVersion};
pub use worker::{Worker, WorkerConfig, WorkerError, WorkerId, WorkerState};
