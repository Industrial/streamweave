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

## Scope

StreamWeave is an **in-process**, graph-based streaming framework. It does not provide distributed execution or fault tolerance; for that, run multiple processes and use external coordination (e.g. Kafka consumer groups, external state stores). See [scope-in-process-no-distributed-fault-tolerance.md](scope-in-process-no-distributed-fault-tolerance.md).
