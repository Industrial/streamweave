# StreamWeave Scope: In-Process, No Distributed Fault Tolerance

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §4.

**Dependencies:** None.

**Implementation status:** See [IMPLEMENTATION-STATUS.md](IMPLEMENTATION-STATUS.md). Complete: README scope, architecture doc, API docs.

---

## 1. Objective and rationale

**Objective:** State clearly that StreamWeave’s current design is **single-process** and does **not** provide distributed execution or distributed fault tolerance out of the box. Set correct expectations and avoid misuse.

**Why it matters:**

- Users and operators need to know where StreamWeave fits relative to Flink, Kafka Streams, Timely, etc.
- Prevents incorrect assumptions (e.g. “it will survive process crash” or “it will scale across machines” without additional design).
- Clarifies what “production” means today: single process, optional durability via external stores and deployment practices.

---

## 2. What to document and where

### 2.1 README (user-facing)

Add a **Scope** or **Limitations** section that states explicitly:

- **StreamWeave is an in-process, graph-based streaming framework.**
- **It does not provide:**
  - Distributed execution (multiple workers/nodes).
  - Built-in fault tolerance (e.g. automatic failover, distributed checkpointing).
  - Cluster membership or coordination.
- **It does provide:**
  - Composable async dataflow graphs in a single process.
  - Logical timestamps and progress tracking (single-graph).
  - Integration points (channels, streams) so that you can feed data from or to external systems (Kafka, databases, etc.).
- **For high availability or scale-out:** Run multiple StreamWeave processes and use external coordination (e.g. Kafka consumer groups, external state stores, load balancers). StreamWeave does not implement these; you compose it with existing infrastructure.

**Suggested wording (draft):**

> ## Scope and limitations
>
> StreamWeave runs in a **single process**. It does not provide distributed execution or distributed fault tolerance. For scale-out or HA, run multiple processes and use external coordination (e.g. Kafka consumer groups, external state stores). See [ARCHITECTURE.md](docs/architecture.md) (or this doc) for details.

### 2.2 Architecture / design doc

Create or extend an architecture document (e.g. `docs/architecture.md` or a “Design” section in the gap doc) that covers:

- **Execution model:** One graph, one process; nodes are async tasks; channels are in-process.
- **Failure model:** If the process crashes, all in-memory state is lost unless persisted externally. There is no built-in replay or recovery from a coordinated checkpoint (future work).
- **Scaling:** Horizontal scaling is achieved by the user (e.g. multiple processes, each with its own graph instance, fed by a partitioned source like Kafka).
- **Comparison:** Short table or bullets vs. Flink, Kafka Streams, Timely (e.g. “Flink: distributed, checkpointing, exactly-once; StreamWeave: in-process, composable graphs, no built-in distribution”).

### 2.3 Code and API

- Where relevant (e.g. in `Graph` or execution APIs), add doc comments that refer to “single-process” or “in-process” so that future contributors and users of the API see the scope.
- Avoid naming or documenting APIs in a way that implies distribution (e.g. “cluster”, “worker”, “failover”) unless they are explicitly “reserved for future use” or “placeholder for distribution.”

### 2.4 Future

When distribution or optional fault tolerance is added, update this scope:

- e.g. “StreamWeave supports an optional distributed mode: …” with clear boundaries (what is fault-tolerant, what is not, what is single-process only).

---

## 3. Detailed checklist for “scope” documentation

| Item | Location | Content |
|------|----------|--------|
| Short scope statement | README | In-process only; no distributed FT; use external coordination for HA/scale. |
| Execution model | docs/architecture.md (or similar) | One process, one graph; nodes = tasks; channels in-process. |
| Failure model | docs/architecture.md | Process crash = state loss unless persisted externally. |
| Scaling | README or docs | User runs multiple processes; external systems (Kafka, etc.) provide partitioning. |
| Comparison | docs/streamweave-gap-analysis-and-implementation-strategy.md or separate | StreamWeave vs. Flink / Kafka Streams / Timely (brief). |
| API docs | Graph, execute_*, etc. | “In-process” where relevant. |

---

## 4. References

- Gap analysis §4 (StreamWeave is not a distributed fault-tolerant engine).
- Gap analysis §14–§16 (future: sharding, checkpointing, auto-scaling) for when scope may change.
