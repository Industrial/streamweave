# Architecture Overview

This document describes the high-level architecture of StreamWeave and key design decisions.

## Graph Structure

StreamWeave uses a **graph-based architecture** where computation is organized as nodes connected by edges:

- **Nodes**: Processing units that consume and produce streams; implement the `Node` trait
- **Edges**: Connections between node ports that route data
- **Port mappings**: For nested graphs, mappings from internal to external ports

## Execution Model

- **Concurrent execution**: By default, each node runs in its own Tokio task; multiple nodes execute concurrently
- **Stream-based**: Nodes receive `InputStreams` (HashMap of port → stream) and return `OutputStreams`
- **Channels**: One channel per edge; bounded mpsc for backpressure
- **Pull-based**: Data flows when downstream nodes consume; external inputs drive the pipeline

## Time and Progress

- **Logical timestamps**: Items can carry `LogicalTime` for ordering and progress tracking (see [logical-timestamps-timely-and-streamweave.md](logical-timestamps-timely-and-streamweave.md))
- **Progress**: When using `execute_with_progress`, the completed frontier advances when items reach sinks (see [progress-tracking.md](progress-tracking.md))
- **Event time vs processing time**: See [event-time-semantics.md](event-time-semantics.md)

## Determinism

**Default behavior:** Execution is **not** deterministic. Nodes run concurrently; the order in which items arrive at multi-input nodes is undefined (e.g. `tokio::select!` over streams). Shared mutable state or non-deterministic operations (e.g. `rand::random()`, `SystemTime::now()`) can produce different outputs across runs.

**Determinism contract (when deterministic mode is used):**

1. **Output order** of a node is deterministic given:
   - The order of items on each of its input ports, and
   - The node's internal logic (no hidden non-determinism)
2. **Input order** is defined by:
   - Graph topology (which edges feed which ports), and
   - Explicit ordering rules (e.g. by logical time, port order, or sequence)
3. **Single-threaded / strict-order mode:** (planned) Run in one task with topological order so that all ordering is defined
4. **Logical-time mode:** (planned) When every item carries logical time and the runtime respects time order, concurrent execution can still be deterministic

**Constructs that break determinism:**

- Unordered merge of multiple inputs (e.g. `select!` with no tie-breaking)
- Shared mutable state without synchronization
- `rand::random()` or `SystemTime::now()` in node logic (unless explicitly documented)
- Non-deterministic scheduling or parallelism

See [deterministic-execution.md](deterministic-execution.md) for the full design.

## Exactly-once state

Stateful nodes that participate in checkpoint/replay should follow the **exactly-once state contract**:

- **Keyed state:** State is addressed by a key (e.g. `user_id`). All updates are (key, value, version).
- **Version / logical time:** Every update carries a version (e.g. `LogicalTime`). Applying the same (key, value, version) again is **idempotent** (no double-apply).
- **Ordering:** Updates for the same key are processed in version order; older versions arriving after newer may be ignored.
- **Persistence:** Backends must support `snapshot` and `restore` for checkpointing.

Implement the [`ExactlyOnceStateBackend`](https://docs.rs/streamweave/*/streamweave/state/trait.ExactlyOnceStateBackend.html) trait for custom state stores. See [exactly-once-state.md](exactly-once-state.md).

## Scope

StreamWeave is an **in-process**, graph-based streaming framework.

- **Execution model:** One graph, one process; nodes run as async tasks; channels are in-process
- **Failure model:** If the process crashes, in-memory state is lost unless persisted externally; no built-in coordinated checkpoint or recovery
- **Scaling:** Horizontal scaling is achieved by the user (e.g. multiple processes, each with its own graph, fed by partitioned sources like Kafka)
- **What it does not provide:** Distributed execution, built-in fault tolerance, cluster membership, or coordination

For scale-out or HA, run multiple processes and use external coordination (e.g. Kafka consumer groups, external state stores). See [scope-in-process-no-distributed-fault-tolerance.md](scope-in-process-no-distributed-fault-tolerance.md).
