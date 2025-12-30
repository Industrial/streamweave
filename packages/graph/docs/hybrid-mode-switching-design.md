# Hybrid Mode Switching Architecture Design

## Overview

This document designs the architecture for dynamic mode switching in StreamWeave's Hybrid execution mode. Hybrid mode starts in in-process zero-copy mode and dynamically switches to distributed mode when throughput exceeds a threshold.

## Requirements

- **Throughput Monitoring**: Track items/second processed across all nodes
- **Threshold Detection**: Switch modes when throughput exceeds `local_threshold`
- **Graceful Transition**: Drain in-process channels, switch to distributed channels, resume processing
- **State Migration**: Ensure no items are lost during transition
- **Metrics & Logging**: Track mode switches, reasons, and performance impact

## Architecture Components

### 1. Throughput Monitor

A component that tracks throughput across the graph:

```rust
pub struct ThroughputMonitor {
    item_count: Arc<AtomicU64>,
    start_time: Instant,
    window_size: Duration,
    samples: VecDeque<(Instant, u64)>, // (timestamp, item_count)
}
```

**Features**:
- Atomic counters for thread-safe item counting
- Rolling window for smoothing spikes
- Configurable window size (e.g., 1 second)
- Calculate items/second from rolling average

### 2. Mode Switch Manager

Manages the mode switching process:

```rust
pub struct ModeSwitchManager {
    current_mode: ExecutionMode,
    target_mode: Option<ExecutionMode>,
    switch_in_progress: Arc<AtomicBool>,
    switch_notify: Arc<Notify>,
}
```

**Responsibilities**:
- Track current execution mode
- Coordinate mode switch process
- Signal nodes to pause during transition
- Manage state migration

### 3. State Migration Handler

Handles migration of in-flight items during mode switch:

```rust
pub struct StateMigrationHandler {
    in_flight_items: Vec<ChannelItem>,
    migration_buffer: VecDeque<ChannelItem>,
}
```

**Responsibilities**:
- Buffer items during transition
- Drain in-process channels
- Serialize items for distributed mode
- Ensure no data loss

## Design Decisions

### 1. Throughput Measurement Strategy

**Option A: Per-Node Counting**
- Each node reports items processed
- GraphExecutor aggregates counts
- Pros: Detailed per-node metrics
- Cons: More overhead, complex aggregation

**Option B: Global Counter**
- Single atomic counter incremented by all nodes
- Simple aggregation
- Pros: Low overhead, simple
- Cons: Less detailed metrics

**Recommendation**: Option B (Global Counter) for simplicity, with optional per-node metrics

### 2. Rolling Average Calculation

**Window-based Rolling Average**:
- Maintain a sliding window of (timestamp, count) pairs
- Calculate average items/second over window
- Window size: 1-5 seconds (configurable)

**Formula**:
```
throughput = (current_count - window_start_count) / window_duration
```

### 3. Mode Switch Trigger

**Threshold Comparison**:
- Calculate rolling average throughput
- Compare against `local_threshold` (items/second)
- Switch when: `rolling_average > local_threshold`

**Hysteresis**:
- Consider adding hysteresis to prevent rapid switching
- Switch to distributed when: `throughput > threshold * 1.1`
- Switch back to in-process when: `throughput < threshold * 0.9`

### 4. Transition Process

**Step 1: Signal Pause**
- Set pause signal to true
- Wait for nodes to finish current items
- Nodes check pause signal before processing each item

**Step 2: Drain Channels**
- Collect all items from in-process channels
- Buffer items for migration
- Ensure channels are empty

**Step 3: Switch Channels**
- Create new distributed channels
- Replace in-process channels with distributed channels
- Update node channel references

**Step 4: Migrate State**
- Serialize buffered items
- Send to new distributed channels
- Resume processing

**Step 5: Resume**
- Clear pause signal
- Nodes resume with new channels
- Continue monitoring throughput

### 5. State Migration Strategy

**Option A: Buffer All Items**
- Collect all in-flight items
- Serialize and send to distributed channels
- Pros: No data loss
- Cons: Memory overhead, potential delay

**Option B: Drop In-Flight Items**
- Drop items during transition
- Resume from next items
- Pros: Simple, fast
- Cons: Data loss

**Recommendation**: Option A (Buffer All Items) - ensure no data loss

### 6. Channel Replacement

**Strategy**:
- Create new distributed channels
- Update `channel_senders` and `channel_receivers` maps
- Nodes will use new channels on next send/recv
- Old channels are dropped (items already drained)

## Integration Points

### 1. GraphExecutor

Add fields to `GraphExecutor`:
```rust
pub struct GraphExecutor {
    // ... existing fields ...
    throughput_monitor: Arc<ThroughputMonitor>,
    mode_switch_manager: Arc<ModeSwitchManager>,
    current_execution_mode: ExecutionMode, // Track actual current mode
}
```

### 2. Node Execution

Nodes increment throughput counter:
```rust
// In ProducerNode, TransformerNode, ConsumerNode
throughput_monitor.increment_item_count();
```

### 3. Background Monitoring Task

Spawn a background task that:
- Periodically checks throughput
- Compares against threshold
- Triggers mode switch if needed

```rust
async fn monitor_throughput(&self) {
    let interval = Duration::from_millis(100); // Check every 100ms
    loop {
        tokio::time::sleep(interval).await;
        
        let throughput = self.throughput_monitor.calculate_throughput();
        if throughput > self.local_threshold {
            self.trigger_mode_switch().await?;
        }
    }
}
```

## Error Handling

- **Switch Failure**: Rollback to previous mode, log error
- **State Migration Failure**: Retry migration, log error
- **Channel Creation Failure**: Return error, don't switch
- **Timeout**: Force switch after timeout, log warning

## Performance Considerations

1. **Monitoring Overhead**: Throughput monitoring should be lightweight
2. **Switch Latency**: Mode switch adds latency, minimize transition time
3. **Memory**: State migration buffers items, monitor memory usage
4. **CPU**: Serialization during migration is CPU-intensive

## Configuration

```rust
pub struct HybridConfig {
    pub local_threshold: usize,        // Items/second threshold
    pub monitoring_interval_ms: u64,   // How often to check (default: 100ms)
    pub window_size_ms: u64,          // Rolling window size (default: 1000ms)
    pub switch_timeout_ms: u64,        // Max time for switch (default: 5000ms)
    pub enable_hysteresis: bool,       // Prevent rapid switching (default: true)
}
```

## Next Steps

1. Implement `ThroughputMonitor`
2. Implement `ModeSwitchManager`
3. Add throughput tracking to nodes
4. Implement mode switch logic
5. Add state migration
6. Add metrics and logging
7. Add tests

## References

- Current hybrid mode implementation: `execution.rs:1716-1780`
- Channel management: `execution.rs:1355-1450`
- Node execution: `node.rs:1080-2720`

