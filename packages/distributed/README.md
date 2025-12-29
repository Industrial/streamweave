# streamweave-distributed

[![Crates.io](https://img.shields.io/crates/v/streamweave-distributed.svg)](https://crates.io/crates/streamweave-distributed)
[![Documentation](https://docs.rs/streamweave-distributed/badge.svg)](https://docs.rs/streamweave-distributed)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Distributed processing integration for StreamWeave**  
*Distribute stream processing across multiple nodes with fault tolerance and partitioning.*

The `streamweave-distributed` package provides distributed processing capabilities for StreamWeave. It enables running stream processing pipelines across multiple worker nodes, with coordinator-based task distribution, fault tolerance, network communication, and automatic rebalancing.

## ✨ Key Features

- **Coordinator**: Central node for managing workers and task distribution
- **Workers**: Processing nodes that execute pipelines
- **Network Communication**: Reliable communication between coordinator and workers
- **Fault Tolerance**: Failure detection, recovery, and task redistribution
- **Partitioning**: Distribute data across workers
- **Rebalancing**: Automatic rebalancing when workers join/leave
- **Checkpointing**: State checkpointing for recovery

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-distributed = "0.3.0"
```

## 🚀 Quick Start

### Coordinator Setup

```rust
use streamweave_distributed::{Coordinator, CoordinatorConfig};
use std::net::SocketAddr;

let config = CoordinatorConfig {
    bind_address: "0.0.0.0:8080".parse().unwrap(),
    heartbeat_timeout: Duration::from_secs(30),
    max_workers: 100,
    ..Default::default()
};

let mut coordinator = CoordinatorImpl::new(config);
coordinator.start().await?;
```

### Worker Setup

```rust
use streamweave_distributed::{Worker, WorkerConfig};

let config = WorkerConfig {
    worker_id: WorkerId::new("worker-1".to_string()),
    coordinator_address: "127.0.0.1:8080".parse().unwrap(),
    max_concurrent_tasks: 10,
    ..Default::default()
};

let mut worker = WorkerImpl::new(config);
worker.register().await?;
worker.start().await?;
```

## 📖 API Overview

### Coordinator Trait

The `Coordinator` trait manages workers and task distribution:

```rust
#[async_trait]
pub trait Coordinator {
    async fn register_worker(&mut self, worker_info: WorkerInfo) -> Result<()>;
    async fn assign_partitions(&mut self, worker_id: &WorkerId, partitions: Vec<usize>) -> Result<()>;
    async fn rebalance(&mut self) -> Result<()>;
    // ... other methods
}
```

### Worker Trait

The `Worker` trait defines worker node behavior:

```rust
#[async_trait]
pub trait Worker {
    async fn register(&mut self) -> Result<()>;
    async fn start(&mut self) -> Result<()>;
    async fn process_task(&mut self, task_id: String, partition: usize) -> Result<()>;
    // ... other methods
}
```

### Partitioning

Distribute data across workers:

```rust
pub trait Partitioner {
    fn partition(&self, item: &T, num_partitions: usize) -> usize;
}
```

### Fault Tolerance

Failure detection and recovery:

```rust
pub struct FailureDetector {
    heartbeat_timeout: Duration,
}

pub struct RecoveryManager {
    checkpoint_interval: Duration,
}
```

## 📚 Usage Examples

### Basic Distributed Setup

Set up coordinator and workers:

```rust
// Coordinator
let coordinator_config = CoordinatorConfig {
    bind_address: "0.0.0.0:8080".parse().unwrap(),
    ..Default::default()
};
let mut coordinator = CoordinatorImpl::new(coordinator_config);
coordinator.start().await?;

// Worker 1
let worker1_config = WorkerConfig {
    worker_id: WorkerId::new("worker-1".to_string()),
    coordinator_address: "127.0.0.1:8080".parse().unwrap(),
    ..Default::default()
};
let mut worker1 = WorkerImpl::new(worker1_config);
worker1.register().await?;
worker1.start().await?;

// Worker 2
let worker2_config = WorkerConfig {
    worker_id: WorkerId::new("worker-2".to_string()),
    coordinator_address: "127.0.0.1:8080".parse().unwrap(),
    ..Default::default()
};
let mut worker2 = WorkerImpl::new(worker2_config);
worker2.register().await?;
worker2.start().await?;
```

### Partition Assignment

Assign partitions to workers:

```rust
// Coordinator assigns partitions
coordinator.assign_partitions(
    &worker1_id,
    vec![0, 1, 2]  // Worker 1 handles partitions 0, 1, 2
).await?;

coordinator.assign_partitions(
    &worker2_id,
    vec![3, 4, 5]  // Worker 2 handles partitions 3, 4, 5
).await?;
```

### Rebalancing

Automatic rebalancing when workers join/leave:

```rust
let config = CoordinatorConfig {
    rebalance_strategy: RebalanceStrategy::Gradual,
    ..Default::default()
};

// When worker joins or leaves, coordinator automatically rebalances
coordinator.rebalance().await?;
```

### Fault Tolerance

Configure failure detection and recovery:

```rust
use streamweave_distributed::fault_tolerance::{FailureDetector, RecoveryManager};

// Failure detection
let detector = FailureDetector::new(Duration::from_secs(30));

// Recovery manager
let recovery = RecoveryManager::new(Duration::from_secs(60));

// Checkpoint state periodically
recovery.checkpoint().await?;
```

### Network Communication

Configure network settings:

```rust
use streamweave_distributed::network::{ConnectionPool, Transport};

// Connection pool
let pool = ConnectionPool::new()
    .with_max_connections(100)
    .with_timeout(Duration::from_secs(30));

// Transport layer
let transport = Transport::new(pool);
```

### Worker Health Monitoring

Monitor worker health:

```rust
// Check worker health
let worker_info = coordinator.get_worker(&worker_id).await?;
if worker_info.is_healthy(Duration::from_secs(30)) {
    // Worker is healthy
} else {
    // Worker may be down, trigger recovery
    coordinator.handle_worker_failure(&worker_id).await?;
}
```

## 🏗️ Architecture

Distributed processing architecture:

```
┌──────────────┐
│ Coordinator  │───manages───> Workers
└──────────────┘     │
                     │
                     ▼
              ┌──────────────┐
              │   Worker 1   │───processes───> Partition 0, 1, 2
              └──────────────┘
                     │
                     ▼
              ┌──────────────┐
              │   Worker 2   │───processes───> Partition 3, 4, 5
              └──────────────┘
```

**Distributed Flow:**
1. Coordinator starts and listens for workers
2. Workers register with coordinator
3. Coordinator assigns partitions to workers
4. Workers process assigned partitions
5. Workers send heartbeats to coordinator
6. Coordinator monitors worker health
7. On failure, coordinator redistributes tasks

## 🔧 Configuration

### Coordinator Configuration

```rust
pub struct CoordinatorConfig {
    pub bind_address: SocketAddr,
    pub heartbeat_timeout: Duration,
    pub health_check_interval: Duration,
    pub max_workers: usize,
    pub rebalance_strategy: RebalanceStrategy,
    pub enable_failover: bool,
}
```

### Worker Configuration

```rust
pub struct WorkerConfig {
    pub worker_id: WorkerId,
    pub bind_address: SocketAddr,
    pub coordinator_address: SocketAddr,
    pub heartbeat_interval: Duration,
    pub communication_timeout: Duration,
    pub max_concurrent_tasks: usize,
    pub assigned_partitions: Vec<usize>,
}
```

### Rebalance Strategies

**Immediate:**
- Rebalance immediately
- May cause temporary disruption
- Fast rebalancing

**Gradual (Default):**
- Migrate partitions one at a time
- Minimal disruption
- Smooth rebalancing

**Manual:**
- No automatic rebalancing
- Manual intervention required
- Full control

## 🔍 Error Handling

Distributed operations return specific error types:

```rust
pub enum CoordinatorError {
    RegistrationFailed(String),
    WorkerNotFound(WorkerId),
    Communication(String),
    InvalidPartition(String),
    RebalanceFailed(String),
}

pub enum WorkerError {
    RegistrationFailed(String),
    Communication(String),
    InvalidTask(String),
    PipelineError(String),
}
```

## ⚡ Performance Considerations

- **Network Overhead**: Communication between coordinator and workers
- **Heartbeat Frequency**: Balance between detection speed and overhead
- **Partition Size**: Smaller partitions = more parallelism, more overhead
- **Rebalancing Cost**: Rebalancing may cause temporary disruption

## 📝 Examples

For more examples, see:
- [Distributed Processing Example](https://github.com/Industrial/streamweave/tree/main/examples)
- [Fault Tolerance Example](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-distributed` depends on:

- `streamweave` - Core traits
- `streamweave-graph` - Graph API
- `tokio` - Async runtime
- `futures` - Future utilities
- `serde` - Serialization
- `serde_json` - JSON serialization
- `chrono` - Timestamp support

## 🎯 Use Cases

Distributed processing is used for:

1. **Horizontal Scaling**: Scale processing across multiple nodes
2. **Fault Tolerance**: Handle node failures gracefully
3. **Load Distribution**: Distribute load across workers
4. **High Throughput**: Process large volumes of data
5. **Elastic Scaling**: Add/remove workers dynamically

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-distributed)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/distributed)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-graph](../graph/README.md) - Graph API
- [streamweave-pipeline](../pipeline/README.md) - Pipeline API

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

