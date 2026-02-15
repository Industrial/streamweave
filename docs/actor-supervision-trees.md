# Actor Supervision Trees

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §6.

**Dependencies:** None for in-process. Mature cluster sharding for distributed worker supervision.

**Implementation status:** See [IMPLEMENTATION-STATUS.md](IMPLEMENTATION-STATUS.md#actor-supervision-treesmd). Phases 1–5 done. Phase 4: `set_supervision_group()`, RestartGroup (shared group restart count), Restart (per-node count), Stop vs Escalate distinct.

---

## Quick start / Example

Use **execute_with_supervision** to run the graph with failure handling; **set_node_supervision_policy** (or **set_supervision_group**) to configure **Restart** or **Escalate**. **Full runnable example:** `cargo run --example supervision_restart` (failure + restart). See [examples/supervision_restart.rs](../examples/supervision_restart.rs).

---

## 1. Objective and rationale

**Objective:** A hierarchy of “supervisors” where each parent supervises children: on child failure, the parent can **restart**, **escalate**, or **stop**. Used in Erlang/Elixir and Akka for fault containment and recovery.

**Why it matters for StreamWeave:**

- In long-running or distributed systems, failure isolation and restart policies are essential.
- Supervision trees give a clear model for “what to do when this node (or subgraph) fails.”
- Without it, a single node panic or fatal error can take down the whole graph or leave the system in an undefined state.

---

## 2. Current state in StreamWeave

- **Nodes:** Run as Tokio tasks; if a node panics or returns a fatal error, the behavior is “error port” and/or graph-level stop (e.g. the task completes with `Err`, and the caller may log or stop).
- **No hierarchy:** There is no notion of “this node is supervised by that parent” or “restart this node on panic.”
- **No policy:** No configurable “restart this node,” “restart this subgraph,” or “stop the graph” policy.

---

## 3. Scope: in-process first

**In-process:** Model **node groups** or **subgraphs** as units of supervision. If a node in the group panics or returns a fatal error, the policy (restart this node, restart the group, stop the graph) is configurable. This is a “light” supervision tree (e.g. one or two levels: graph supervises nodes or subgraphs).

**Rust-native:** Use Tokio’s task semantics and a small layer: wrap node execution in a task that catches panic and reports to a “supervisor” that can restart or escalate. No need to adopt the full actor model (mailboxes, location transparency) unless desired.

**Distributed (later):** When you have workers, each worker can run a supervision tree locally; worker failure is handled by the cluster layer (restart worker, rebalance). This doc focuses on in-process.

---

## 4. Detailed design

### 4.1 Supervision unit

**Unit of supervision:** A single **node** or a **subgraph** (group of nodes). The supervisor treats the unit as one “child”: if the child fails, the policy applies to that whole unit.

- **Node as unit:** One node = one task; on panic/error, the supervisor decides: restart node, restart sibling nodes, or stop graph.
- **Subgraph as unit:** A set of nodes that run together (e.g. a `Graph` used as a node); on any node failure inside the subgraph, the policy applies to the whole subgraph (e.g. restart the whole subgraph).

### 4.2 Failure detection

- **Panic:** Use `std::panic::catch_unwind` or run the node in a task and `join`; on panic, report to supervisor with a “panic” reason.
- **Fatal error:** Node returns `Err` from `execute`; the execution layer already sees this. Treat as “failure” and report to supervisor (with error kind if available).
- **Distinguish:** Optional: “transient” vs “fatal” (e.g. via error type or a flag) so that the policy can “retry” vs “stop.”

### 4.3 Policy (what the supervisor does)

Configurable per unit (or global default):

| Policy | Meaning |
|--------|--------|
| **Restart** | Restart this node (or subgraph) with the same inputs/state as before; optionally with a backoff and max retries. |
| **Restart group** | Restart this node and its siblings (same parent). |
| **Stop** | Do not restart; stop this unit and notify parent (graph may then stop or continue without this branch). |
| **Escalate** | Notify parent supervisor (e.g. graph); parent applies its own policy. |

**Defaults:** e.g. “restart node up to 3 times with 1s backoff; then escalate to graph; graph stops.”

### 4.4 Supervisor implementation sketch

- **Supervisor handle:** A small struct or channel that the execution layer holds. When a node task finishes with `Err` or panic, the execution layer sends a message: `(node_id, error, maybe_restart_count)`.
- **Supervisor task:** A dedicated task (or a method run on a timer or on event) that:
  - Receives failure notifications.
  - Looks up policy for that node (or its group).
  - If restart: spawns a new task for that node (re-run `execute` with the same or fresh inputs; may need to “reconnect” channels or re-create the node from a factory).
  - If stop: marks the node as stopped; optionally notifies the rest of the graph (e.g. close channels).
  - If escalate: sends to parent (e.g. graph-level “node X failed permanently”).

### 4.5 Restart semantics

- **State:** On restart, the node starts with **fresh state** (no in-memory state from before the crash) unless state was persisted and restored. So “restart” is “run the node again from scratch” for that unit.
- **Inputs:** The channels feeding the node may have buffered data; the restarted node will see data that arrives after restart. Replay of “missed” data is not automatic unless the source supports replay and the execution layer reconnects from a checkpoint (see exactly-once and checkpointing).
- **Outputs:** Downstream nodes may have already consumed some output from the failed node; after restart, the node will produce new output. Ordering and exactly-once are not guaranteed across restarts unless you have exactly-once state and checkpointing.

### 4.6 API surface

**Configuration:**

```rust
pub struct SupervisionPolicy {
    pub on_failure: FailureAction,  // Restart | RestartGroup | Stop | Escalate
    pub max_restarts: Option<u32>,
    pub restart_backoff: std::time::Duration,
}

impl Graph {
    /// Set policy for a node (or use default).
    pub fn set_node_supervision_policy(&mut self, node_id: &str, policy: SupervisionPolicy) { ... }
}
```

**Observability:** Emit events or logs on “node X failed,” “node X restarted,” “graph stopped due to escalation,” so that operators can see supervision in action.

---

## 5. Implementation phases

| Phase | Status | Content |
|-------|--------|---------|
| **1** | Done | `SupervisionPolicy`, `FailureAction` in `src/supervision.rs`. |
| **2** | Done | `FailureReport`, `enable_failure_reporting()`, `wait_for_completion` sends on failure. |
| **3** | Done | `execute_with_supervision()` graph-level restart. |
| **4** | ❌ Not done | RestartGroup/Escalate distinct; per-node/group restart; subgraph-as-unit. | “restart group” and “escalate” so that subgraph or graph can be stopped. |
| **5** | Done | Tests: test_execute_with_supervision_*. |

---

## 6. References

- Gap analysis §6 (Actor supervision trees).
- Tokio: task spawning, `JoinHandle`, panic propagation.
- Erlang/Akka: supervision tree concepts (one-way inspiration; no need to replicate fully).
