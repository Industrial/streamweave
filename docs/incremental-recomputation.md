# Incremental Recomputation

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §13.

**Dependencies:** Timestamped differential dataflow (full incremental); or Progress tracking (for time-range recomputation).

---

## 1. Objective and rationale

**Objective:** When inputs change (e.g. new data, updated source), **only recompute** the minimal set of downstream work that depends on that change, instead of re-running the entire pipeline. This improves efficiency and scalability.

**Why it matters:**

- Full recomputation does not scale for large or long-running pipelines.
- Incremental recomputation is what makes “always-on” views and stream processing practical (e.g. Materialize, Differential Dataflow).

---

## 2. Current state in StreamWeave

- **Execution:** “Run the graph”; there is no notion of “what changed” or “recompute only affected nodes.”
- **No diff:** Streams do not carry retractions or deltas; everything is “full value.”
- **Restart:** To “update,” users typically restart the graph or replay from a checkpoint; no fine-grained incremental behavior.

---

## 3. Three levels of incremental behavior

### 3.1 Short term: patterns and node-level caching

**Goal:** Document patterns and optionally add node-level caching so that **unchanged** inputs can be skipped.

**Approaches:**

- **Restart only this subgraph:** Document how to isolate a subgraph and re-run only that part when its inputs change (e.g. by reconnecting inputs from a new source or from a checkpoint).
- **Node-level memoization:** For pure or keyed nodes, cache output keyed by input identity (e.g. hash of input). When input hasn’t changed, skip recomputation and reuse cached output. Useful for expensive, deterministic nodes.
- **Replay from checkpoint:** Combined with exactly-once state, replay from last checkpoint so that only “new” data is processed; state updates are idempotent so replay does not double-apply.

**Implementation:** Mostly documentation and optional helpers (e.g. a “memoizing” wrapper node). No change to core execution model.

### 3.2 Medium term: timestamped differential dataflow

**Goal:** If data is `(payload, time, diff)`, then **incremental recomputation is the default behavior** of differential operators.

- **Differential group-by, join, etc.:** Only propagate **changes** (deltas). Downstream sees only what changed at each logical time.
- **No “full recompute”:** The operator state is maintained incrementally; new input (or retractions) produce only new output diffs.
- **Tie-in:** See [timestamped-differential-dataflow.md](timestamped-differential-dataflow.md). Once that is in place, “incremental recomputation” for differential nodes is inherent.

**Implementation:** Implement differential operators; document that they are incremental by construction.

### 3.3 Long term: progress and time-range recomputation

**Goal:** Use **progress tracking** and **dependency tracking** to drive “which nodes need to run for this time range.”

- **Idea:** If we know “input has changed only for time range [T1, T2],” then only nodes that depend on that input and that have not yet produced output for [T1, T2] need to run. Other nodes can reuse previous output.
- **Requires:** Clear dependency graph (which node reads which output); progress (completed time) per node or per edge; and a scheduler that can “re-run node N for time range [T1, T2]” and merge results. This is advanced and likely requires deeper execution model changes (e.g. time-scoped execution, checkpoint/restore per time range).
- **Defer:** Until basic differential and progress are solid.

---

## 4. Detailed design (short term)

### 4.1 Memoizing node wrapper

**API:** A wrapper that takes a node and an optional cache (e.g. in-memory or with TTL):

- **Key:** Hash of (input port names, input chunk or key, optional logical time range).
- **Value:** Output produced for that input.
- **Behavior:** Before running the node, check cache; if hit, emit cached output. If miss, run node, store result in cache, emit.

**Limitations:** Only valid for **deterministic** nodes; cache size and invalidation (e.g. when source is “replayed”) must be defined. Useful for expensive map/filter/aggregate that is keyed and deterministic.

### 4.2 Replay from checkpoint

- **Checkpoint:** Persist state and input positions (see [distributed-checkpointing.md](distributed-checkpointing.md)).
- **Replay:** After restore, replay input from last checkpoint. With **exactly-once state** (see [exactly-once-state.md](exactly-once-state.md)), replay does not double-apply; only “new” data (after checkpoint) is effectively processed. So “incremental” here means “only process the delta since last checkpoint,” not “only process changed keys.”

### 4.3 Documentation

- Document “incremental patterns”: replay from checkpoint, memoization, and (when available) differential operators.
- Document that **differential nodes** are incremental by design once [timestamped-differential-dataflow.md](timestamped-differential-dataflow.md) is implemented.

---

## 5. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | Document incremental patterns (replay, memoization, subgraph restart). |
| **2** | (Optional) Provide a memoizing wrapper node or trait for deterministic, keyed nodes. |
| **3** | With differential dataflow: document and rely on incremental behavior of differential operators. |
| **4** | (Later) Progress + dependency-based time-range recomputation. |

---

## 6. References

- Gap analysis §13 (Incremental recomputation).
- [timestamped-differential-dataflow.md](timestamped-differential-dataflow.md), [progress-tracking.md](progress-tracking.md), [exactly-once-state.md](exactly-once-state.md), [distributed-checkpointing.md](distributed-checkpointing.md).
