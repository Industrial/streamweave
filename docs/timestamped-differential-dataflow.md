# Timestamped Differential Dataflow

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §10.

**Dependencies:** Logical timestamps (implemented).

---

## 1. Objective and rationale

**Objective:** Dataflow where every record carries a **logical timestamp** and a **multiplicity (diff)** (insert +1, retract −1). Computation is expressed as **incremental** operators (e.g. differenced join/aggregate) so that only **changes** are propagated and outputs are maintained as differences over time. This yields efficient incremental and reactive updates.

**Why it matters:**

- In Timely/Differential, **all streams use and build on this model**; it is the base construct.
- Without it, “update the pipeline” usually means full recomputation or ad-hoc state.
- With it, you get predictable, scalable incremental behavior and a clear path to “only recompute what changed.”

---

## 2. Current state in StreamWeave

- No first-class `(data, time, diff)` representation; streams are `Arc<dyn Any + Send + Sync>` (or `Timestamped<T>` with time only).
- No diff, no retractions, no incremental operators.
- Logical timestamps exist; they can be extended to “timestamped + differential.”

---

## 3. Core concepts

### 3.1 Difference (diff)

- Each record has a **multiplicity** (integer): **+1** = insert, **−1** = retract. Optionally **+k** / **−k** for batch updates.
- A “collection” at logical time T is the multiset of (data, diff) where diff is the net count for that (data, T). Sum of diffs per (data, T) gives the current count (e.g. 0 = deleted, 1 = present, 2 = duplicate).

### 3.2 Incremental operators

- **Differenced group-by:** Input (key, value, time, diff); output (key, aggregate, time, diff) where output diffs are the **change** in the aggregate for that key at that time.
- **Differenced join:** Inputs (K, V1, time, diff) and (K, V2, time, diff); output (K, (V1, V2), time, diff) only for **changes** to the join result (i.e. new or retracted pairs).
- **Accumulation:** Downstream can “integrate” diffs to get a current snapshot (sum diffs per key up to current time).

### 3.3 Timestamped differential as the base

- Every item on a stream is `(payload, LogicalTime, diff)` (or an enum Insert/Retract with time).
- The runtime and built-in operators are defined over this type.
- Legacy `Arc<dyn Any>` can be wrapped with default timestamp and diff = +1 for compatibility.

---

## 4. Detailed design

### 4.1 Core type

**Option A – Triple:**

```rust
pub struct DifferentialElement<T> {
    pub payload: T,
    pub time: LogicalTime,
    pub diff: i64,  // +1 insert, -1 retract, etc.
}
```

**Option B – Enum:**

```rust
pub enum DifferentialMessage<T> {
    Insert(Timestamped<T>),   // diff +1
    Retract(Timestamped<T>), // diff -1
    Update { old: Timestamped<T>, new: Timestamped<T> }, // or two Retract+Insert
}
```

Recommendation: **Triple** for simplicity and alignment with Differential Dataflow; operators can map to/from enum if needed.

### 4.2 Execution path

- **New path:** “Differential execution” where channels carry `DifferentialElement<Payload>` (or `(Payload, LogicalTime, i64)`). Nodes that are “differential-aware” consume and produce this type.
- **Compatibility:** Existing nodes that expect `Payload` (or `Timestamped<Payload>`) get a **wrapper** that:
  - Inbound: strips diff, presents only (payload, time); or presents “current snapshot” by integrating diffs (more complex).
  - Outbound: wraps node output in (payload, time, +1). So non-differential nodes run as “insert-only” sources of differential streams.

### 4.3 Canonical operators (nodes)

Implement a small set of **differential** nodes:

- **Differential group-by (reduce):** Input stream of (key, value, time, diff); output stream of (key, aggregate, time, diff) where aggregate is e.g. sum/count and output diffs are the **delta** at each time.
- **Differential join:** Two inputs (K, V1, time, diff) and (K, V2, time, diff); output (K, (V1, V2), time, diff) as the **changes** to the join result.
- **Differential map/filter:** Pass-through with same time and diff; filter can drop (payload, time, diff) or emit retractions to cancel previous.

Implementation of these operators requires internal state (e.g. indexes per key and time) and is a significant effort; start with one (e.g. group-by) as a reference.

### 4.4 Fixed-point iterations (advanced)

Differential Dataflow supports **nested timestamps** and **iterative fixed point** (e.g. until no more changes). That is a larger extension (rounds, feedback, “no more data at this round”). Defer until basic differential streams and one or two operators are in place.

### 4.5 Pragmatics

- **Start small:** Single-worker, single-graph; timestamped differential streams and one or two key operators (group-by, join).
- **Scale and distribution** can follow once the model is proven in-process.
- Keep the existing Node API as a compatibility layer (inject default timestamp and diff = +1).

---

## 5. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | Define `DifferentialElement<T>` (or equivalent) and add a “differential” execution path (channels carry (payload, time, diff)). **Done:** `DifferentialElement<T>` and `DifferentialStreamMessage<T>` in `src/time.rs`. |
| **2** | Wrap existing nodes so that they produce (payload, time, +1); consume (payload, time, diff) by stripping diff for non-differential nodes or by integrating for snapshot. **Done:** `ToDifferentialNode` in `src/nodes/stream/to_differential_node.rs` wraps each input as `DifferentialElement::insert(payload, time, +1)`; accepts Timestamped, StreamMessage, or plain `Arc<dyn Any>` (monotonic time). |
| **3** | Implement **differential group-by** (single key, sum or count; output diffs as deltas). **Done:** `DifferentialGroupByNode` in `src/nodes/reduction/differential_group_by_node.rs`; count aggregation with key extractor; outputs (key, count, time, diff). |
| **4** | Implement **differential join** (equi-join; output changes only). **Done:** `DifferentialJoinNode` in `src/nodes/differential_join_node.rs`; consumes `DifferentialStreamMessage` from left/right, emits (key, (V1,V2), time, diff). |
| **5** | Document and test; consider incremental recomputation (see [incremental-recomputation.md](incremental-recomputation.md)) as the default behavior of differential operators. |

---

## 6. References

- Gap analysis §10 (Timestamped differential dataflow).
- [logical-timestamps-timely-and-streamweave.md](logical-timestamps-timely-and-streamweave.md).
- Differential Dataflow (Materialize/Timely): semantics and operators.
- [incremental-recomputation.md](incremental-recomputation.md).
