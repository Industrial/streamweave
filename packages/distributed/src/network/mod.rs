//! Network communication layer for distributed processing.
//!
//! Provides connection management, streaming data transfer, connection pooling,
//! and retry logic for coordinator-worker and worker-worker communication.

pub mod connection;
pub mod discovery;
pub mod pool;
pub mod transport;

pub use connection::{Connection, ConnectionError, ConnectionManager, ConnectionState};
pub use discovery::{DiscoveryError, NodeDiscovery, NodeInfo};
pub use pool::{ConnectionPool, ConnectionPoolConfig, PoolError};
pub use transport::{TcpStreamTransport, TransportError, TransportMessage};
