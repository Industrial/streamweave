# True Cyclic Iterative Dataflows

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §5.

**Dependencies:** None for Option A (bounded iteration). Logical timestamps / rounds for Option B (Timely-style).

---

## 1. Objective and rationale

**Objective:** Support graphs that contain **cycles** (feedback edges) and execute until a fixed point or a bounded number of rounds. Used for iterative algorithms (e.g. PageRank, iterative refinement, recursive rules).

**Why it matters:**

- Without cycles, “keep updating until convergence” cannot be expressed inside the same graph.
- Users otherwise unroll loops manually (fixed iterations) or move iteration outside the engine; both are brittle and less composable.
- Cyclic dataflow is a core feature of Timely Dataflow and similar systems.

---

## 2. Current state in StreamWeave

- **Topology:** The graph is a DAG in structure (edges connect nodes); the current execution model is pull-based stream composition.
- **Cycles:** If the user adds an edge that creates a cycle (e.g. A → B → C → A), behavior is undefined or may deadlock: there is no notion of “round” or “iteration” or buffered feedback.
- **No round/iteration:** No first-class concept of “round 1,” “round 2,” or “feedback from round N into round N+1.”

---

## 3. Two implementation paths

### Option A: Bounded iteration (minimal)

**Idea:** Add a **bounded iteration** construct: a subgraph with one or more “feedback” edges that are **buffered per round**. Execution runs round 1, then feeds designated outputs back into designated inputs, then round 2, up to a max iteration or until a “converged” signal.

**Characteristics:**

- Can be implemented with current stream abstractions.
- No change to the core “all streams are timestamped” model required (but can use a round index as a simple “time”).
- A “cycle” or “iteration” node (or subgraph wrapper) that:
  - Accepts “initial input” and “feedback” inputs.
  - For round 1: runs the subgraph with initial input only (feedback empty).
  - For round 2..N: runs the subgraph with feedback = output from previous round; stops when max rounds or converged.

**Design elements:**

- **IterationNode or CyclicSubgraph:** Wraps an inner graph. Exposes:
  - “Seed” or “initial” input port.
  - “Feedback” input port (filled from previous round’s output).
  - “Output” port (feeds next round’s feedback and/or external consumer).
  - “Converged” or “done” signal (optional): a boolean or token that stops iteration.
- **Round buffer:** Between rounds, the output of the inner graph is collected (or streamed) and fed back as the “feedback” input for the next round. No data flows “around the cycle” within the same round; the cycle is broken by round boundaries.
- **Max iterations:** A cap (e.g. `max_rounds: usize`) to avoid infinite loops.

**API sketch:**

```rust
/// Runs `inner_graph` for up to `max_rounds`. Each round: feed `feedback` from previous round's output;
/// round 1 uses only `seed` (feedback empty). Stops when `max_rounds` or when `converged` signal is produced.
pub struct BoundedIterationNode {
    inner_graph: Graph,
    max_rounds: usize,
    // Port names: "seed", "feedback", "output", "converged"
}
```

### Option B: Logical timestamps / rounds (Timely-style)

**Idea:** Allow cycles in the graph; every edge carries a **round index** (or logical time). The engine only delivers to a node when **all** inputs for that round are available. So “round” becomes part of the logical time (e.g. `(epoch, round)`), and the dataflow engine schedules work by round.

**Characteristics:**

- Aligns with logical timestamps and progress tracking (see [logical-timestamps-timely-and-streamweave.md](logical-timestamps-timely-and-streamweave.md)).
- Single unified model: no special “iteration node”; cycles are natural.
- Requires:
  - Logical time to include a “round” or to be used as round (e.g. round = time / 1000).
  - Buffering: for a node with multiple inputs, do not deliver round R until all inputs have at least one item for round R (or a frontier that advances per round).
  - Feedback edges: the output of a node at round R is delivered to the downstream node as round R (or R+1); the downstream node might be the same node (cycle) or another. For a cycle, the “next” round’s input is the output of the “previous” round.

**Design elements:**

- **Logical time = (epoch, round) or single u64 as round:** So that “advance to round 2” is “advance_to(LogicalTime::new(2))”.
- **Progress per round:** The completed frontier for a cycle component is “all inputs have been seen up to round R”; then the engine can run round R+1.
- **Fixed point:** Iteration stops when no data is produced on a round (or when a “done” message is produced). The engine can detect “no output this round” and then advance the frontier and consider the cycle done for that epoch.

**Implementation outline:**

- Reuse/extend `LogicalTime` to represent round (or use a pair (epoch, round)).
- In `run_dataflow`, for nodes that have a cycle (feedback edge): the channel from the “feedback” source is the same as the channel that delivers the previous round’s output. So the runtime must:
  - Associate each message with a round.
  - For the feedback input, only deliver messages when they are for the “current” round and when the round has been triggered (e.g. when all other inputs for that round are ready).
- This is a deeper change to the execution layer; it is the right long-term direction if you aim for Timely-style semantics.

---

## 4. Detailed design (Option A – bounded iteration)

### 4.1 Control flow

1. **Round 0 (or “seed”):** Feed seed input into the inner graph’s “seed” port; leave “feedback” port empty or with a special “first round” marker.
2. **Round k (k ≥ 1):** Feed the output from round k−1 into the “feedback” port. Run the inner graph until it produces output (or a “converged” signal).
3. **Stop when:** `k == max_rounds` or the inner graph emits “converged” (or “no more data” for a round, depending on convention).

### 4.2 Inner graph interface

The inner graph must have:

- At least one input that receives “seed” or “feedback” (or both merged).
- At least one output that is fed to the next round and/or to the outer consumer.
- Optional: an output or signal that indicates “converged” (e.g. a control message or a dedicated port).

### 4.3 Buffering between rounds

- Option A1: **Full buffer.** Run the inner graph to completion for round k (all outputs consumed), collect all output items, then feed them as the feedback stream for round k+1. Simple but memory-heavy for large outputs.
- Option A2: **Streaming feedback.** The inner graph’s output stream is directly connected to the feedback input of the same logical “iteration” node; the iteration driver ensures that the feedback stream is not read until the previous round is complete (e.g. by switching streams or by tagging items with round and only delivering round k to the inner graph when round k−1 is done). This avoids materializing the full round in memory if the inner graph can stream.

### 4.4 Integration with existing Graph

- **As a Node:** `BoundedIterationNode` implements `Node`. Its `execute` runs the inner graph in a loop (round 1..max_rounds), using a channel or stream to pass feedback. The inner graph is a `Graph` that implements `Node` (graph-as-node); so the iteration node holds a graph and runs it repeatedly.
- **As a graph-level primitive:** Alternatively, the `Graph` struct could have a method “add_cyclic_region” that marks a set of nodes and feedback edges and runs them with round semantics. That would require the execution layer to understand “cyclic regions” and round boundaries.

---

## 5. Bounded iteration API (design)

This section specifies the API for phase 1. Implementation (phase 2) must follow this contract.

### 5.1 Ports

| Port        | Direction | Type                     | Description                                                                 |
|-------------|-----------|--------------------------|-----------------------------------------------------------------------------|
| `seed`      | Input     | `Arc<dyn Any + Send + Sync>` | Initial input for round 1. Stream of items fed once at start.           |
| `feedback`  | Input     | (internal)               | Filled by the node from the previous round's `output`. Not wired externally.|
| `output`    | Output    | `Arc<dyn Any + Send + Sync>` | Items produced each round. Forwarded to next round and to external sink.|
| `converged` | Output    | `Arc<dyn Any + Send + Sync>` | Optional. When the inner graph emits here, iteration stops early.        |

**Note:** `feedback` is an internal port of the inner graph. The iteration driver connects the previous round's `output` stream to the inner graph's `feedback` input before each round k ≥ 1.

### 5.2 Inner graph contract

The inner graph (a `Graph` used as a subgraph) must:

1. **Inputs:** Have at least `seed` and `feedback` input ports. Names must match exactly.
2. **Outputs:** Have at least `output` port. Optional `converged` port.
3. **Merge semantics:** The inner graph typically merges `seed` (round 1 only) and `feedback` (rounds 2..N) into a single logical stream. A merge/sync node or similar can combine them.

### 5.3 BoundedIterationNode constructor

```rust
/// Runs an inner graph for up to `max_rounds`. Round 1: seed only. Rounds 2..N: feedback from previous output.
pub struct BoundedIterationNode {
    inner_graph: Graph,
    max_rounds: usize,
}

impl BoundedIterationNode {
    pub fn new(name: String, inner_graph: Graph, max_rounds: usize) -> Self;
}
```

**Parameters:**

- `name`: Node name for the iteration node.
- `inner_graph`: The subgraph to run each round. Must have `seed`, `feedback`, `output`, and optionally `converged` ports.
- `max_rounds`: Maximum iterations. Must be ≥ 1.

### 5.4 Execution semantics

1. **Round 1:**
   - Connect external `seed` stream to inner graph's `seed` input.
   - Provide an **empty** stream to inner graph's `feedback` input (no items).
   - Run the inner graph to completion (all streams drained or graph stops).
   - Collect all items from `output` into a buffer. Emit them to external `output` consumers.
   - If `converged` emits any item, stop iteration immediately.

2. **Round k (2 ≤ k ≤ max_rounds):**
   - Disconnect or finish the previous round's connections.
   - Connect the **buffered output from round k−1** as a stream to inner graph's `feedback` input.
   - Provide an **empty** stream to `seed` (seed is only used in round 1).
   - Run the inner graph to completion.
   - Buffer output; emit to external consumers.
   - If `converged` emits, stop iteration.

3. **Stop conditions:**
   - `k == max_rounds`, or
   - Inner graph emits on `converged`, or
   - (Future) Inner graph produces no output for a round (fixed point).

### 5.5 Buffering strategy (phase 2)

Use **full buffer** (Option A1): collect all output items for round k before starting round k+1. Simpler and correct for MVP. Streaming feedback can be added later.

### 5.6 Integration note

`BoundedIterationNode` wraps a `Graph` and drives it in a loop. The current `Graph` API uses `run_dataflow` with connected channels. For iteration, the driver must either: (a) run the graph once per round with fresh channel bindings (seed/feedback for that round), or (b) use a graph execution API that supports "run until round boundary" if such an API is introduced. Phase 2 will implement (a) using the existing `connect_input_channel` and collecting output from exposed output ports.

---

## 6. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | Design and document the bounded iteration API (rounds, seed, feedback, max_rounds, converged). **Done:** §5 above. |
| **2** | Implement `BoundedIterationNode` (or equivalent) that runs an inner graph for multiple rounds with buffered feedback. |
| **3** | Add tests: e.g. simple fixed-point (x := x/2 + 1 until stable), PageRank-style stub. |
| **4** | (Later) Option B: integrate rounds with logical time and allow cycles in the main graph with round-based delivery. |

---

## 7. References

- Gap analysis §5 (True cyclic iterative dataflows).
- [logical-timestamps-timely-and-streamweave.md](logical-timestamps-timely-and-streamweave.md) for logical time and progress (Option B).
- Timely Dataflow: rounds and feedback edges.
