# Incremental Recomputation

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §13.

**Dependencies:** Timestamped differential dataflow (full incremental); or Progress tracking (for time-range recomputation).

**Implementation status:** See [IMPLEMENTATION-STATUS.md](IMPLEMENTATION-STATUS.md). Partial: MemoizingMapNode, replay-from-checkpoint, differential operators done; time-range recomputation deferred.

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

### 4.3 Incremental patterns (practical guide)

This section documents how to achieve incremental behavior today and with planned features.

#### 4.3.1 Replay from checkpoint

**When to use:** After a restart or when you need to reprocess only the "delta" since a known position.

**How it works:**
1. Periodically persist checkpoint (state + input positions) to durable storage.
2. On restart, restore state from the last checkpoint.
3. Replay input from the saved position. With **exactly-once state** (idempotent updates), replay does not double-apply; only data after the checkpoint is effectively processed.

**Result:** Incremental in the sense of "only process the delta since last checkpoint," not per-key. Useful for crash recovery and catch-up after downtime.

**References:** [distributed-checkpointing.md](distributed-checkpointing.md), [exactly-once-state.md](exactly-once-state.md).

#### 4.3.2 Memoization (node-level caching)

**When to use:** For expensive, **deterministic**, keyed nodes (e.g. map, filter, aggregation by key). When the same input produces the same output, cache the result and skip recomputation.

**Key design:**
- **Cache key:** Hash of (input port values, optional logical time or key).
- **Cache value:** Output produced for that input.
- **Behavior:** Before running the node, check cache; on hit, emit cached output. On miss, run node, store result, emit.

**Limitations:** Only valid for deterministic nodes. Cache size and invalidation must be defined (e.g. TTL, LRU, or explicit invalidate on replay). Not suitable for stateful nodes that accumulate across inputs.

**Implementation:** Optional memoizing wrapper node (see phase 2). Wrap a pure/keyed node to add caching.

#### 4.3.3 Subgraph restart

**When to use:** When only a **subset** of the graph needs to be re-run (e.g. one branch's source changed).

**How it works:**
1. Isolate the affected subgraph (e.g. a `Graph` used as a node).
2. Disconnect its inputs from the old source and connect to a new source (or replay stream).
3. Run only that subgraph (or the minimal set of nodes downstream of the change).

**Note:** StreamWeave's execution model runs the whole graph. Fine-grained "restart only this subgraph" may require manual composition (e.g. a parent graph that conditionally feeds a child graph) or future execution-layer support.

#### 4.3.4 Differential operators (incremental by design)

**When to use:** For pipelines that process collections with inserts and retractions.

**How it works:** Differential nodes (`DifferentialGroupByNode`, `DifferentialJoinNode`, etc.) consume `(payload, time, diff)` and emit only **changes** (deltas). They maintain incremental state; new input produces only new output diffs. No full recompute.

**References:** [timestamped-differential-dataflow.md](timestamped-differential-dataflow.md). Differential dataflow is implemented; use `ToDifferentialNode`, `DifferentialGroupByNode`, `DifferentialJoinNode` for incremental behavior.

---

## 5. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | Document incremental patterns (replay, memoization, subgraph restart). **Done:** §4.3. |
| **2** | (Optional) Provide a memoizing wrapper node or trait for deterministic, keyed nodes. **Done:** MemoizingMapNode with MemoizeKeyExtractor (IdentityKeyExtractor, HashKeyExtractor). |
| **3** | With differential dataflow: document and rely on incremental behavior of differential operators. **Done.** |
| **4** | Progress + dependency-based time-range recomputation. **Implemented:** `plan_recompute`, `RecomputePlan`, `Graph::plan_recompute`, `Graph::execute_recompute`; per-sink frontiers via `ProgressHandle::from_sink_frontiers` / `sink_frontiers()`. Subgraph time-scoped execution deferred. See §5.1. |

### 5.1 Phase 4 implementation details (time-range recomputation)

**Dependency tracking:** `Graph::nodes_depending_on(node)` (direct dependents), `Graph::nodes_downstream_transitive(node)` (transitive downstream).

**Types (`incremental` module):** `TimeRange { from, to }`, `RecomputeRequest { time_range, source_node }`, `RecomputePlan { nodes }`.

**Scheduler:** `plan_recompute(request, downstream_fn, all_nodes, sink_frontiers, sink_keys)` returns which nodes need recomputation. Use with `Graph::plan_recompute` and optional `ProgressHandle` from `execute_with_progress_per_sink`.

**Per-sink progress:** `ProgressHandle::from_sink_frontiers(frontiers, keys)` and `sink_frontiers()` expose completed logical time per (node, port). `execute_with_progress_per_sink` returns a handle with keys for recompute planning.

**Execution:** `Graph::execute_recompute(plan)` runs the full graph. `Graph::execute_for_time_range(plan, time_range)` runs the nodes in the plan; when plan equals all nodes it runs the full graph; when it is a subset it falls back to full graph. Strict subgraph-only execution and time-filtered inputs are future work.

---

## 6. References

- Gap analysis §13 (Incremental recomputation).
- [timestamped-differential-dataflow.md](timestamped-differential-dataflow.md), [progress-tracking.md](progress-tracking.md), [exactly-once-state.md](exactly-once-state.md), [distributed-checkpointing.md](distributed-checkpointing.md).
