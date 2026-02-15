# Research Document: Gap Analysis and Implementation Strategy

## StreamWeave vs. Mature Streaming/Dataflow Systems

**Scope:** For each capability listed below, this document states the **objective**, **reason to care for StreamWeave**, and **how best to implement it** (or whether to defer/avoid), given StreamWeave’s current in-process, graph-based, async-stream architecture.

---

## Timely Dataflow and timestamped differential as the base

In **Timely Dataflow**, the core abstraction is that every message on a stream is a **timestamped data item** (and in **Differential Dataflow**, each item has a **multiplicity** so that updates are represented as +1/−1). All built-in operators (map, filter, join, group_by, etc.) are defined over `(data, time)` (and in Differential, `(data, time, diff)`). There is no “raw” stream type that bypasses timestamps: timestamps are part of the execution model. Progress tracking, scheduling, and cycles (rounds) are all expressed in terms of that logical time. So in Timely/Differential, **all streams use and build on timestamped (differential) dataflow**; it is the base construct.

**Could StreamWeave use this as the base too?** Yes. Making timestamped (and optionally differential) streams the **base** would align StreamWeave with Timely’s model: every item would carry a logical timestamp (and optionally a diff), the runtime would maintain progress, and operators would be defined over (data, time) or (data, time, diff). That would give a single, consistent foundation for event-time, windowing, cycles, incremental recomputation, and determinism. The tradeoff is a breaking or opt-in change: existing `Arc<dyn Any>` streams would either be wrapped in a default “timestamp = 0” / “diff = +1” envelope, or kept behind a legacy API. Recommendation: introduce a **first-class base type** (e.g. `StreamElement<T> { data: T, time: LogicalTime, diff: i64 }`) and a graph execution path that uses it; keep the current API as a compatibility layer that injects a default timestamp/diff so existing graphs keep working while new code can rely on the unified base.

---

The following sections are **ordered by dependency**: each item lists its dependencies (or “none”). Implement in this order to respect prerequisites.

---

## 1. Logical timestamps

**Dependencies:** None.

**Deep dive:** See [Logical timestamps: Timely and StreamWeave](logical-timestamps-timely-and-streamweave.md) for how Timely implements timestamps and how to apply that technique in StreamWeave.

**Objective:** A notion of “time” or “round” that is part of the dataflow model (e.g. (data, timestamp) or (data, round)), used to order work, define progress, and support cycles and incremental computation.

**Reason for inclusion:** Logical timestamps are the glue between event-time semantics, progress tracking, deterministic execution, and iterative/cyclic dataflow. Without them, the engine cannot reason about “what has been processed up to when.”

**Implementation in StreamWeave:**

- **Today:** No first-class logical time; time is only where a node explicitly adds it (e.g. system time).
- **Best path:**
  - Introduce a **logical time type** (e.g. `u64` or a small struct with epoch/round) and a standard envelope: `Message<T> { payload: T, logical_time: LogicalTime }` (or reuse/extend existing `Message<T>` if present).
  - Have the **graph runtime** (or a wrapper layer) attach/advance logical time for each node so that downstream can rely on it.
  - Use the same type for “event time” when the source provides it, and for “round” in iterative graphs.
- **Pragmatics:** This is a cross-cutting change. Do it behind a feature flag or in a dedicated “advanced” API so existing graphs keep working.

---

## 2. Deterministic execution

**Dependencies:** None (optional: Logical timestamps for time-based determinism).

**Objective:** For the same logical input and graph, execution produces the same outputs (and ideally same observable side effects) regardless of scheduling, parallelism, or run order. Essential for testing, debugging, and reproducibility.

**Reason for inclusion:** Non-determinism makes bugs hard to reproduce and makes exactly-once and correct aggregation much harder. Determinism is a prerequisite for serious production guarantees.

**Implementation in StreamWeave:**

- **Today:** Execution is concurrent (multiple nodes running); ordering between nodes is not guaranteed. Any use of shared mutable state or non-deterministic ordering (e.g. `select!` over multiple inputs) can break determinism.
- **Best path:**
  - **Define “determinism contract”:** e.g. “output order of a node is deterministic given the order of its inputs; input order is defined by graph topology and explicit ordering rules.”
  - **Single-threaded execution mode:** Run the graph with a single task or strict topological order so that all ordering is defined. Useful for tests and debugging.
  - **Logical timestamps:** If every item has a logical time and the runtime respects time order (e.g. process in timestamp order per key), then concurrent execution can still be deterministic.
  - **Avoid:** Non-deterministic constructs in core library (e.g. unordered fan-in unless explicitly labeled “unordered”).
- **Pragmatics:** Start with a “deterministic execution” mode (single-threaded or fixed schedule) and document the contract; then add logical-time-based determinism as the model matures.

---

## 3. Exactly-once state

**Dependencies:** None.

**Objective:** Each update to internal state is applied exactly once, and on recovery the state is consistent (no double-apply, no lost updates). Usually achieved with idempotent updates, deterministic replay, and checkpointing.

**Reason for inclusion:** Users expect “counts and aggregates are correct” after failures and restarts. At-least-once or at-most-once is insufficient for financial or analytical correctness.

**Implementation in StreamWeave:**

- **Today:** PRD mentions exactly-once as a goal (transactions, idempotency, dedup, checkpointing, offset tracking); implementation is not present.
- **Best path:**
  - **State identity:** Stateful nodes should have a clear notion of “key” and “version” or “sequence” so that replay of the same input produces the same state update.
  - **Idempotent state updates:** Design state APIs so that applying the same (key, update, logical time) multiple times is idempotent.
  - **Checkpoint and replay:** Combine with checkpointing: after restore, replay from last checkpoint; exactly-once state ensures state is not double-applied.
  - **Sinks:** Exactly-once sinks require idempotent writes or transactional writes (e.g. “write with key/version; overwrite same key is idempotent”).
- **Pragmatics:** This is largely independent of distribution. Implement for in-process stateful nodes first; then extend to distributed state stores.

---

## 4. StreamWeave is not a distributed fault-tolerant engine

**Dependencies:** None.

**Objective:** To state clearly that StreamWeave’s current design is single-process and does not provide distributed fault tolerance out of the box.

**Reason for inclusion:** Sets correct expectations and avoids misuse. It also clarifies where the product stands relative to Flink, Kafka Streams, Timely, etc.

**Implementation in StreamWeave:**

- **Best path:** **Document explicitly** in README and architecture docs: “StreamWeave is an in-process, graph-based streaming framework. It does not provide distributed execution or fault tolerance; for that, run multiple processes and use external coordination (e.g. Kafka consumer groups, external state stores).”
- **Future:** If you add distribution (see next items), update this to “optional distributed mode” with clear boundaries (what is fault-tolerant, what is not).

---

## 5. True cyclic iterative dataflows

**Dependencies:** None (Option A: bounded iteration). Logical timestamps (Option B: rounds as logical time).

**Objective:** Graphs that contain cycles (e.g. “feedback” edges) and execute until a fixed point (or a bounded number of rounds). Used for iterative algorithms (e.g. PageRank, iterative refinement, recursive rules).

**Reason for inclusion:** Without cycles, you cannot express “keep updating until convergence” inside the same graph. Users either unroll loops manually (fixed iterations) or move iteration outside the engine; both are brittle and less composable.

**Implementation in StreamWeave:**

- **Today:** The graph is a DAG; execution is pull-based stream composition. There is no notion of “round” or “iteration” or feedback.
- **Best path:**
  - **Option A (minimal):** Add a **bounded iteration** construct: a subgraph with one or more “feedback” edges that are buffered per round. Execution runs round 1, then feeds designated outputs back into designated inputs, then round 2, up to a max iteration or until a “converged” signal.
  - **Option B (deeper):** Introduce **logical timestamps / rounds** (see above). Cycles are allowed; each edge carries a round index; the engine only delivers to a node when all inputs for that round are available.
- **Pragmatics:** Option A can be implemented with current stream abstractions (e.g. a “cycle” node that multiplexes “initial input” vs “feedback” and drives rounds). Option B aligns with logical timestamps and progress tracking and is the right long-term direction if you aim for Timely-style semantics.

---

## 6. Actor supervision trees

**Dependencies:** None (in-process). Mature cluster sharding (for distributed worker supervision).

**Objective:** A hierarchy of actors where each parent supervises children: on child failure, the parent can restart, escalate, or stop. Used in Erlang/Elixir and Akka for fault containment and recovery.

**Reason for inclusion:** In distributed or long-running systems, failure isolation and restart policies are essential. Supervision trees give a clear model for “what to do when this node fails.”

**Implementation in StreamWeave:**

- **Today:** Nodes run as tasks; failure handling is mostly “error port” and graph-level stop. No hierarchy of supervisors.
- **Best path:**
  - **In-process:** Model **node groups** or **subgraphs** as units of supervision: if a node in the group panics or returns a fatal error, the policy (restart this node, restart the group, stop the graph) is configurable. This is a “light” supervision tree (one level: graph supervises nodes or subgraphs).
  - **Rust-native:** Use tokio’s task semantics and a small layer: wrap node execution in a task that catches panic and reports to a “supervisor” that can restart or escalate. No need to adopt the full actor model unless you want mailboxes and location transparency.
  - **Distributed:** If you later have workers, each worker can run a supervision tree locally; worker failure is handled by the cluster layer (restart worker, rebalance).
- **Pragmatics:** Start with “restart this node on panic” and “restart this subgraph on fatal error”; grow into a small, explicit supervision API rather than a full Akka-style stack.

---

## 7. Production-hardened cluster tooling

**Dependencies:** None (single-process hardening). Mature cluster sharding (for cluster-level tooling).

**Objective:** Operational features for running the system in production: deployment, config management, health checks, metrics, logging, upgrades, and failure procedures.

**Reason for inclusion:** Without these, “cluster” means “a bunch of processes” rather than something operators can run safely at scale.

**Implementation in StreamWeave:**

- **Today:** No cluster; tooling is minimal (likely some tests and maybe basic observability).
- **Best path:**
  - **Now:** Harden **single-process** first: health endpoint, readiness (e.g. “graph started”), structured logging, metrics (throughput, latency, error rate per node or port). OpenTelemetry/Prometheus align with your PRD.
  - **With distribution:** Add cluster-level health (e.g. “all shards running”), config distribution (e.g. from env or a simple config service), graceful shutdown (drain in-flight, then exit), and rollback-friendly deployment (e.g. version in binary or config).
  - **Avoid:** Building a full PaaS. Prefer integration with Kubernetes, systemd, or existing deployment tools.
- **Pragmatics:** Production hardening is incremental. Each new capability (distribution, checkpointing, sharding) should come with the minimal operational hooks (health, metrics, config) needed to run it safely.

---

## 8. Event-time semantics

**Dependencies:** Logical timestamps.

**Objective:** Use the time **attached to the event** (e.g. “when the click happened”) as the basis for ordering, windowing, and progress, rather than processing time (“when we saw the event”). This matches user intuition and is required for correct analytics with late or out-of-order data.

**Reason for inclusion:** Processing-time-only semantics are wrong for late data and backfill. Event-time is the standard in modern stream systems (Flink, Beam, Kafka Streams, etc.).

**Implementation in StreamWeave:**

- **Today:** There is a `TimestampNode` that adds **processing time** (SystemTime::now()). There is no first-class event-time field or semantics; windowing is count-based (fixed-size batches).
- **Best path:**
  - **Event time as first-class:** Define a convention or trait for “events that carry event time” (e.g. `event_time_ms: u64` or extractor from payload).
  - **Event-time windowing:** Implement tumbling/sliding/session windows keyed by **event time** (see “Windowing” and “Event-time processing” below).
  - **Watermarks:** To know “no more data before time T,” introduce watermarks (see “Progress tracking”).
- **Pragmatics:** Keep processing-time nodes for backward compatibility; add event-time as an optional, explicit path (e.g. `EventTimeTimestampNode`, event-time window nodes).

---

## 9. Progress tracking

**Dependencies:** Logical timestamps.

**Objective:** The runtime (or operators) can answer: “For this input/output, what is the minimum logical timestamp that has been completed (e.g. all data with time ≤ T has been processed)?” That is the “progress” or “watermark.” It enables safe closing of event-time windows and correct flushing.

**Reason for inclusion:** Without progress, you cannot close windows in event-time correctly (you might close too early or never). It is required for event-time processing and for backpressure and scheduling.

**Implementation in StreamWeave:**

- **Today:** No progress or watermark concept.
- **Best path:**
  - **Per-node progress:** Each node can expose “minimum timestamp I have completed” (e.g. from the minimum logical time of items it has processed and propagated).
  - **Graph-level progress:** For a single graph, progress is the minimum over sink nodes or a designated “progress” output. For DAGs, progress is the minimum over nodes that have seen all inputs up to that time.
  - **Watermarks as messages:** Optionally, allow sources (or a coordinator) to inject watermark messages (e.g. “progress is now T”); operators that buffer by time can then flush windows up to T.
- **Pragmatics:** Start with a single-worker, single-graph progress: “minimum logical time completed.” Distributed progress (e.g. across shards) comes later with distribution.

---

## 10. Timestamped differential dataflow

**Dependencies:** Logical timestamps.

**Objective:** Dataflow where every record carries a logical timestamp and a multiplicity (insert +1, retract −1). Computation is expressed as incremental operators (e.g. differenced join/aggregate) so that only changes are propagated and outputs are maintained as differences over time. This yields efficient incremental and reactive updates.

**Reason for inclusion:** It is the backbone of systems like Materialize/Timely/Differential. In Timely/Differential, **all streams use and build on this model**; it is the base construct. Without it, “update the pipeline” usually means full recomputation or ad-hoc state. With it, you get predictable, scalable incremental behavior and a clear path to “only recompute what changed.”

**Implementation in StreamWeave:**

- **Today:** StreamWeave has no first-class (data, time, diff) representation; streams are `Arc<dyn Any + Send + Sync>` with no shared time or diff model.
- **Best path:** Use timestamped (and optionally differential) streams as the **base** construct, as in Timely: every item carries logical time (and optionally diff); the runtime and operators are defined over this type. Concretely:
  - Define a core type, e.g. `(Arc<Payload>, LogicalTime, Diff)` (or a small enum for “insert/retract/update”) and make it the **default** envelope for graph execution; legacy `Arc<dyn Any>` can be wrapped with a default timestamp/diff for compatibility.
  - Implement a small set of **canonical differential operators** (e.g. differenced group-by, join) as nodes that consume and produce this format.
  - Keep the existing Node API available as a compatibility layer that injects default timestamp/diff so current users are unaffected.
- **Pragmatics:** Full differential engine (fixed-point iterations, nested timestamps, efficient shared state) is a large undertaking. Start with single-worker, single-graph timestamped differential streams and a few key operators; scale and distribution can follow once the model is proven in-process.

---

## 11. Windowing

**Dependencies:** None (count-based, processing-time windows). Event-time semantics, Progress tracking (event-time windows).

**Objective:** Group events into finite windows (tumbling, sliding, session, or custom) for aggregation or join. Windows can be defined by count, processing time, or event time.

**Reason for inclusion:** Most stream analytics (rates, counts per minute, sessions, etc.) are expressed as windows. Count-only windows are limiting; time-based (especially event-time) windows are expected in mature systems.

**Implementation in StreamWeave:**

- **Today:** `WindowNode` is **count-based** (fixed number of items per batch). No time-based or event-time windows.
- **Best path:**
  - **Keep count-based window:** Already exists; document as "count window."
  - **Add time-based windows:**
    - **Processing-time windows:** Emit a new window every N ms (wall clock); assign each item to the current window. Simple, deterministic per run.
    - **Event-time windows:** Assign items to windows by event time; close a window when the watermark shows "no more data for this window." Requires event-time + watermarks (Progress tracking).
  - **Window types:** Tumbling (fixed, non-overlapping), sliding (fixed size, smaller step), session (gap-based). Start with tumbling; add sliding and session when event-time and watermarks exist.
- **Pragmatics:** Event-time windowing depends on event-time semantics and progress tracking; implement those first, then add event-time window nodes.

---

## 12. Event-time processing

**Dependencies:** Event-time semantics, Progress tracking, Windowing (event-time).

**Objective:** End-to-end processing where ordering, windowing, and progress are based on event time, with handling of late data (e.g. drop, buffer, or side output).

**Reason for inclusion:** This is the combination of event-time semantics, logical timestamps, progress tracking, and event-time windowing. It is what users mean by "correct streaming with late data."

**Implementation in StreamWeave:**

- **Best path:** Implement the pieces above: event-time as first-class, logical timestamps, progress/watermarks, event-time windows. Then add a **late-data policy** (e.g. discard, buffer until watermark + allowed lateness, or side output). Event-time processing is the integration of these pieces rather than one extra feature.

---

## 13. Incremental recomputation

**Dependencies:** Timestamped differential dataflow; or Progress tracking.

**Objective:** When inputs change (e.g. new data, updated source), only recompute the minimal set of downstream work that depends on that change, instead of re-running the entire pipeline.

**Reason for inclusion:** Efficiency and scalability. Full recomputation does not scale for large or long-running pipelines; incremental recomputation is what makes "always-on" views and stream processing practical.

**Implementation in StreamWeave:**

- **Today:** Execution is "run the graph"; there is no notion of "what changed" or "recompute only affected nodes."
- **Best path:**
  - **Short term:** Document patterns (e.g. "restart only this subgraph," "replay from checkpoint") and possibly add **node-level caching or memoization** keyed by input identity/hash so that unchanged inputs can be skipped.
  - **Medium term:** Tie into **timestamped differential dataflow**: if data is (payload, time, diff), then "incremental recomputation" is the default behavior of differential operators.
  - **Long term:** **Progress tracking** plus dependency tracking can drive "which nodes need to run for this time range."

---

## 14. Mature cluster sharding

**Dependencies:** Exactly-once state.

**Objective:** Partition the logical graph (or data) across multiple workers so that each worker runs a subset of state and computation; routing is by key or partition. State is local to the shard that owns the key.

**Reason for inclusion:** Horizontal scaling and locality: large state and high throughput require sharding. "Mature" means stable partitioning, rebalancing, and state migration.

**Implementation in StreamWeave:**

- **Today:** No sharding; one graph, one process.
- **Best path:**
  - **First:** Define a **partitioning contract** in the API (e.g. "this node's input is partitioned by key K"). The same graph can then be instantiated N times with different partition keys or key ranges.
  - **Second:** Add a **distribution layer**: a driver that deploys N graph instances (processes or threads), routes input by partition key, and optionally merges outputs. External systems (Kafka, etc.) can provide the partitioning.
  - **Mature:** Rebalancing (e.g. when adding/removing workers) requires state migration and exactly-once state so that no keys are lost or duplicated.
- **Pragmatics:** Do after in-process semantics (event-time, progress, exactly-once state) are solid. Sharding is the first step toward "distributed fault-tolerant engine."

---

## 15. Distributed checkpointing

**Dependencies:** Local checkpointing (in-process), Exactly-once state, Mature cluster sharding.

**Objective:** Periodically persist a consistent snapshot of the entire distributed computation (operator state, in-flight positions, etc.) so that on failure the system can restore and resume from that snapshot without duplicate or lost work.

**Reason for inclusion:** Fault tolerance and exactly-once in distributed settings depend on coordinated checkpoints (e.g. Chandy–Lamport or aligned checkpoints).

**Implementation in StreamWeave:**

- **Today:** No distribution, no checkpointing.
- **Best path:**
  - **In-process first:** Add **local checkpointing** for stateful nodes: persist state (and optionally input positions) to durable storage at configured intervals; on restart, restore and resume. This gives single-process fault tolerance and is a prerequisite for distributed checkpointing.
  - **Distributed later:** When you have a distributed runtime (workers, sharding), add a **coordinated checkpoint protocol**: a coordinator triggers "take checkpoint at logical time T"; workers snapshot state and report completion; once all report, the checkpoint is committed.
- **Pragmatics:** Do not add distributed checkpointing before you have distribution and exactly-once state. Order: local state snapshotting → exactly-once state and idempotent sinks → then distributed checkpointing.

---

## 16. Auto-scaling clusters

**Dependencies:** Mature cluster sharding, Production-hardened cluster tooling.

**Objective:** The runtime adds or removes workers (or replicas) based on load, backlog, or policy, and rebalances work (e.g. partitions, shards) across the new set.

**Reason for inclusion:** Operational flexibility and cost efficiency in cloud environments. Not required for correctness, but expected in "production" cluster tooling.

**Implementation in StreamWeave:**

- **Today:** No clusters; single process.
- **Best path:** **Defer** until you have a distributed runtime (workers, sharding, state placement). Auto-scaling then becomes: metrics (throughput, lag, CPU) → scaling policy → add/remove nodes → rebalance shards and state. Consider Kubernetes HPA/VPA and custom controllers; avoid building a full orchestrator inside StreamWeave.
- **Pragmatics:** Low priority until distribution exists. Document as a future goal.

---

## Suggested implementation order

Sections 1–16 above are already in **dependency order**. A practical sequence:

1. **Foundation (no distribution):** Logical timestamps (§1) → Event-time semantics (§8) → Progress tracking (§9) → Deterministic execution mode (§2).
2. **Correctness and state:** Exactly-once state (§3), local checkpointing (in-process), Windowing event-time (§11), Event-time processing (§12).
3. **Advanced in-process:** Timestamped differential dataflow (§10), Cyclic iterative dataflows (§5), Incremental recomputation (§13).
4. **Distribution:** Document current scope (§4), Mature cluster sharding (§14) → Distributed checkpointing (§15) → Supervision (§6) and Production tooling (§7).
5. **Later:** Auto-scaling (§16), “mature” rebalancing and state migration.

This order keeps the current in-process, composable design intact while adding capabilities in dependency order and avoids building distribution before the semantic foundation (time, progress, state) is in place.

---

## Detailed design documents

Each remaining functionality has a dedicated design document in this directory with objectives, current state, detailed design, implementation phases, and references:

| § | Topic | Document |
|---|--------|----------|
| 2 | Deterministic execution | [deterministic-execution.md](deterministic-execution.md) |
| 3 | Exactly-once state | [exactly-once-state.md](exactly-once-state.md) |
| 4 | Scope (in-process, no distributed FT) | [scope-in-process-no-distributed-fault-tolerance.md](scope-in-process-no-distributed-fault-tolerance.md) |
| 5 | Cyclic iterative dataflows | [cyclic-iterative-dataflows.md](cyclic-iterative-dataflows.md) |
| 6 | Actor supervision trees | [actor-supervision-trees.md](actor-supervision-trees.md) |
| 7 | Production cluster tooling | [production-cluster-tooling.md](production-cluster-tooling.md) |
| 8 | Event-time semantics | [event-time-semantics.md](event-time-semantics.md) |
| 9 | Progress tracking (watermarks) | [progress-tracking.md](progress-tracking.md) |
| 10 | Timestamped differential dataflow | [timestamped-differential-dataflow.md](timestamped-differential-dataflow.md) |
| 11 | Windowing | [windowing.md](windowing.md) |
| 12 | Event-time processing | [event-time-processing.md](event-time-processing.md) |
| 13 | Incremental recomputation | [incremental-recomputation.md](incremental-recomputation.md) |
| 14 | Cluster sharding | [cluster-sharding.md](cluster-sharding.md) |
| 15 | Distributed checkpointing | [distributed-checkpointing.md](distributed-checkpointing.md) |
| 16 | Auto-scaling clusters | [auto-scaling-clusters.md](auto-scaling-clusters.md) |

§1 (Logical timestamps) is implemented; see [logical-timestamps-timely-and-streamweave.md](logical-timestamps-timely-and-streamweave.md) for the design and `src/time.rs` for the implementation.
