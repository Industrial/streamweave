# Deterministic Execution

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §2.

**Dependencies:** None (optional: Logical timestamps for time-based determinism).

**Implementation status:** See [IMPLEMENTATION-STATUS.md](IMPLEMENTATION-STATUS.md#deterministic-executionmd). All phases done.

---

## Quick start / Example

To get reproducible, same-order execution (e.g. for tests or debugging), set the execution mode to **Deterministic** before running the graph:

```rust
use streamweave::graph::{ExecutionMode, Graph};

let mut graph = Graph::new("my_graph".to_string());
// ... add nodes and edges ...

graph.set_execution_mode(ExecutionMode::Deterministic);
// Then run the graph (e.g. graph.execute().await?).
// Same inputs produce the same output order every time.
```

With **Concurrent** (the default), node tasks run in parallel and ordering is not guaranteed. Use **Deterministic** when you need reproducible runs. **Full runnable example:** `cargo run --example deterministic_execution` (set_execution_mode(Deterministic), execute_deterministic). See [examples/deterministic_execution.rs](../examples/deterministic_execution.rs).

---

## 1. Objective and rationale

**Objective:** For the same logical input and graph structure, execution produces the same outputs (and ideally the same observable side effects) regardless of scheduling, parallelism, or run order. This is essential for testing, debugging, and reproducibility.

**Why it matters for StreamWeave:**

- Non-determinism makes bugs hard to reproduce and complicates exactly-once and correct aggregation.
- Determinism is a prerequisite for serious production guarantees and for users who need reproducible analytics or compliance.
- A clear determinism contract allows users to reason about when outputs are stable and when they are not.

---

## 2. Current state in StreamWeave

- **Execution model:** Multiple nodes run concurrently as Tokio tasks; ordering between nodes is not guaranteed.
- **Channels:** One channel per edge; backpressure via bounded mpsc. No ordering guarantee across multiple inputs to the same node.
- **Input order:** When a node has multiple input ports, the order in which items arrive from different edges is undefined (e.g. `tokio::select!` or merged streams).
- **Shared state:** Any use of shared mutable state (e.g. a global or shared `HashMap`) without synchronization or deterministic ordering can break determinism.
- **Time:** Logical timestamps are now available; they can be used to define a deterministic order (process by time, then by a secondary key).

---

## 3. Determinism contract (target)

Define a clear contract that users and the runtime can rely on:

1. **Output order of a node** is deterministic given:
   - The order of items on each of its input ports, and
   - The node’s internal logic (no hidden non-determinism, e.g. no `rand::random()` unless explicitly documented as non-deterministic).
2. **Input order** is defined by:
   - Graph topology (which edges feed which ports), and
   - Explicit ordering rules: e.g. “all inputs are ordered by logical time; ties broken by port order or sequence number.”
3. **Single-threaded / strict order mode:** When enabled, the runtime executes nodes in a fixed order (e.g. topological) and processes items in a well-defined order so that the whole run is reproducible.
4. **Logical-time mode:** When every item carries a logical time and the runtime respects time order (e.g. process in timestamp order per key or globally), concurrent execution can still be deterministic.

**Non-deterministic constructs:** Any construct that is explicitly “unordered” (e.g. “unordered fan-in”) should be documented as such so users do not rely on order.

---

## 4. Detailed design

### 4.1 Single-threaded / strict-order execution mode

**Goal:** Run the graph with a single task or a strict schedule so that all ordering is defined and reproducible.

**Options:**

- **Option A – Single task, topological order:**  
  - One async task drives the whole graph.  
  - For each “step,” pull from sources in a fixed order (e.g. by node name), then call each node’s logic in topological order, then deliver to downstream channels.  
  - Pros: Simple, fully deterministic. Cons: No concurrency; throughput limited by single task.

- **Option B – Multiple tasks, deterministic merge at inputs:**  
  - Nodes still run in separate tasks.  
  - For each node with multiple inputs, use a **deterministic merge**: e.g. a single stream that is the merge of all input streams with a defined tie-breaking rule (e.g. by `(LogicalTime, port_index, sequence)`).  
  - Nodes receive one ordered stream (or one ordered stream per port).  
  - Pros: Concurrency preserved. Cons: Requires consistent merge semantics and possibly buffering.

**Recommended first step:** Implement Option A as a “deterministic execution” mode (e.g. `run_dataflow_deterministic` or a `ExecutionMode::Deterministic` flag) that runs in a single task with topological order. Use it in tests and for debugging.

**API sketch:**

```rust
// In graph.rs or execution config
pub enum ExecutionMode {
    /// Default: concurrent tasks per node; order not guaranteed.
    Concurrent,
    /// Single task, topological order; deterministic.
    Deterministic,
}

impl Graph {
    pub fn set_execution_mode(&mut self, mode: ExecutionMode) { ... }
    // Or: execute_deterministic(&mut self) -> ...
}
```

**Implementation details:**

- Reuse existing `run_dataflow` topology (edges, channels) but drive execution from one task.
- Maintain one “current” frontier or round: e.g. for each source/output, track the next item to process.
- In each “tick”: for each node in topological order, if all inputs have data for the current “step,” consume one item from each input (or the minimum logical time across inputs), call the node, push outputs to channels.
- For nodes with no inputs (sources), “produce one item” per tick in a defined order (e.g. by node name, then by item index).
- This requires a synchronous or batched view of the graph: e.g. pull-based, process one “round” of items at a time.

### 4.2 Logical-time-based determinism (concurrent but deterministic)

**Goal:** Allow multiple tasks while preserving determinism by defining order via logical time.

**Idea:** If every item has a logical time and the runtime guarantees that:

- No node observes an item with time `t` before it has observed all items with time `< t` (or all items with time `<= t` from its inputs), and  
- Outputs are produced with the same logical time as the input (or a well-defined rule),

then the observable output order is determined by logical time. Concurrency is allowed as long as time order is respected.

**Requirements:**

- All items carry a logical time (already true when using `execute_with_progress` and the timestamped path).
- Nodes that merge multiple inputs must merge in a deterministic way (e.g. by `(time, port, seq)`).
- Stateful nodes must be deterministic given the sequence of (time, key, value) they see.

**Implementation outline:**

- In the execution layer, when delivering items to a node with multiple inputs, merge streams by `(LogicalTime, port_index, sequence)` so that the node always sees the same order for the same inputs.
- Optionally, buffer items by time and only deliver to the node when “all inputs have reached at least time T” (like Timely’s frontier-based delivery).

### 4.3 Avoiding non-determinism in the core library

- **Fan-in:** Do not use unordered `select!` as the only merge strategy for multi-input nodes if determinism is desired. Provide a “deterministic merge” that orders by (time, port, seq).
- **Random / system time:** Do not use `rand` or `SystemTime::now()` inside core library execution unless explicitly documented as non-deterministic (e.g. “processing-time source”).
- **Documentation:** Document which nodes and which execution modes are deterministic and under what conditions.

### 4.4 Testing determinism

- **Reproducibility test:** Run the same graph with the same inputs multiple times (e.g. in deterministic mode); assert that the sequence of outputs is identical.
- **Concurrent vs deterministic:** For a small graph, run in both modes and compare outputs when the concurrent run happens to be deterministic (e.g. single path, no merge); they should match.
- **Fuzz:** Generate random graphs and inputs; in deterministic mode, run twice and compare outputs.

---

## 5. Implementation phases

| Phase | Status | Content |
|-------|--------|---------|
| **1** | Done | Determinism contract in README, architecture. |
| **2** | Done | `ExecutionMode::Deterministic`, `execute_deterministic()`. |
| **3** | Done | `test_execute_deterministic_reproducible`. |
| **4** | Done | `MergeNode::new_deterministic()`. |

---

## 6. References

- Gap analysis §2 (Deterministic execution).
- Gap analysis §1 (Logical timestamps) for time-based determinism.
- [logical-timestamps-timely-and-streamweave.md](logical-timestamps-timely-and-streamweave.md) for timestamp and progress concepts.
