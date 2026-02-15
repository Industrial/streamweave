# Distributed Checkpointing

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §15.

**Dependencies:** Local checkpointing (in-process), Exactly-once state, Mature cluster sharding.

---

## 1. Objective and rationale

**Objective:** Periodically persist a **consistent snapshot** of the entire distributed computation (operator state, in-flight positions, etc.) so that on failure the system can **restore** and **resume** from that snapshot without duplicate or lost work.

**Why it matters:**

- Fault tolerance and exactly-once in distributed settings depend on coordinated checkpoints (e.g. Chandy–Lamport or aligned checkpoints).
- Without checkpoints, a failure forces full replay from the beginning or from an external source, and state may be inconsistent.

---

## 2. Current state in StreamWeave

- **No checkpointing:** No snapshot, no restore, no coordinated protocol.
- **No distribution:** Single process; distributed checkpointing applies only after sharding exists.
- **State:** In-memory; no durable state format for recovery.

---

## 3. Prerequisites (order of work)

1. **Local checkpointing (in-process):** Persist state (and optionally input positions) of stateful nodes to durable storage at configured intervals; on restart, restore and resume. Single process only. See below.
2. **Exactly-once state:** So that replay after restore does not double-apply (see [exactly-once-state.md](exactly-once-state.md)).
3. **Mature cluster sharding:** So that there are multiple workers to coordinate (see [cluster-sharding.md](cluster-sharding.md)).
4. **Distributed checkpointing:** Coordinate checkpoints across workers so that the snapshot is globally consistent.

---

## 4. Local checkpointing (in-process) – do first

### 4.1 Scope

- **Stateful nodes:** Each stateful node (or the state backend it uses) can **snapshot** its state to bytes (e.g. serialize to disk or to a blob store).
- **Positions:** Optionally, record **input positions** (e.g. Kafka offsets, or logical time up to which input has been consumed) so that on restore we know from where to replay.
- **Trigger:** Checkpoint at **intervals** (e.g. every N seconds) or at **logical time boundaries** (e.g. “when progress reaches T, checkpoint state and record T as the checkpoint position”).
- **Storage:** Configurable backend: local file, S3, etc. Checkpoint is identified by a **checkpoint id** (e.g. logical time T or sequence number).

### 4.2 API (sketch)

- **State backend:** `snapshot() -> Vec<u8>`, `restore(&[u8])`. Already part of exactly-once state design.
- **Graph or execution:** `trigger_checkpoint() -> CheckpointId` (or async). Collects snapshots from all stateful nodes and writes positions; returns id.
- **On startup:** `restore_from_checkpoint(checkpoint_id)`. Load state into nodes; set “resume from position P.” Then start execution; sources replay from P (or from last committed offset if using Kafka, etc.).

### 4.3 Consistency (single process)

- **Quiesce or consistent point:** To get a consistent snapshot, either (a) quiesce the graph (drain in-flight, then snapshot), or (b) use a consistent logical time T and snapshot state “as of T” (all nodes snapshot state that reflects only input up to T). (b) requires support for “snapshot at time T” in state backends (e.g. versioned state).
- **Simple approach:** Quiesce (pause inputs, wait for drain), snapshot all state and positions, then resume. Easiest to reason about.

### 4.4 Failure and recovery

- **Process crash:** On restart, load latest checkpoint, restore state, resume from recorded positions. Replay input from that position; exactly-once state ensures no double-apply.
- **No checkpoint:** If no checkpoint exists, start from scratch (or from user-defined “initial” state).

---

## 5. Distributed checkpointing – later

### 5.1 Goal

All workers in the cluster take a **coordinated** checkpoint so that the **global** state (all shards’ state + in-flight positions) is consistent at a single logical time (or barrier).

### 5.2 Protocols (high level)

- **Chandy–Lamport (global snapshot):** A coordinator sends a “marker” through the graph; when a node receives a marker on all inputs, it snapshots its state and forwards markers. Result: a consistent cut of the distributed state. No single “logical time” required; markers define the cut.
- **Aligned checkpoint (barrier):** All workers agree on a **barrier** (e.g. logical time T). All workers flush in-flight data up to T, snapshot state as of T, and record positions. When all have completed, the checkpoint is committed. Requires that all workers can reach “time T” (progress propagation).

### 5.3 Coordinator

- **Role:** Triggers “take checkpoint at time T” (or “start Chandy–Lamport”). Waits for all workers to report “checkpoint done.” Then marks the checkpoint as committed and can notify workers to release pre-checkpoint state (e.g. truncate logs).
- **Failure during checkpoint:** If a worker fails while checkpointing, the coordinator may abort the checkpoint and retry; or the checkpoint may be partial (only successful workers). Recovery then uses the previous full checkpoint plus replay.

### 5.4 State migration and rebalance

- When **rebalancing** (see [cluster-sharding.md](cluster-sharding.md)), state migration uses the same snapshot/restore primitives: export state for keys K from shard A, send to shard B, shard B restores. Checkpoints can be taken before and after rebalance to ensure consistency.

### 5.5 What to avoid

- Do not add distributed checkpointing **before** you have distribution and exactly-once state. Order: local state snapshotting → exactly-once state and idempotent sinks → then distributed checkpointing.

---

## 6. Coordinated checkpoint protocol (phase 4)

This section documents the protocol for coordinated checkpoints across N workers. StreamWeave does not implement this yet; it is a specification for future work.

### 6.1 Prerequisites

- **N graph instances** (shards), each with `ShardConfig` (see [cluster-sharding.md](cluster-sharding.md)).
- **Shared checkpoint storage** (e.g. S3, distributed filesystem) that all workers can read/write.
- **Coordinator** process or service that knows the worker set and can send barriers and collect reports.

### 6.2 Protocol: barrier-based aligned checkpoint

1. **Coordinator** assigns a checkpoint id (e.g. sequence number) and barrier (e.g. logical time T or barrier id).
2. **Coordinator** sends “take checkpoint &lt;id&gt; at barrier &lt;T&gt;” to all workers (e.g. via HTTP, RPC, or a shared queue).
3. **Each worker:**
   - Waits until its graph has processed input up to T (or drains in-flight if no logical time).
   - Calls `trigger_checkpoint(storage)` (or equivalent) to snapshot its nodes.
   - Writes snapshots to shared storage under `<base>/<id>/shard_<shard_id>/`.
   - Reports “checkpoint done” to the coordinator.
4. **Coordinator** waits for all N workers to report (with timeout).
5. **Commit:** If all reported, coordinator marks checkpoint id as committed (e.g. writes `<base>/<id>/COMMITTED` or updates a registry).
6. **Abort:** If any worker fails or times out, coordinator marks the checkpoint as aborted. Workers may retain their partial snapshots for debugging; the next checkpoint will overwrite. Recovery uses the previous committed checkpoint.

### 6.3 Protocol: Chandy–Lamport (marker-based)

Alternative for graphs without a global logical time:

1. **Coordinator** sends a “marker” (checkpoint id) to all workers.
2. **Each worker** runs the Chandy–Lamport algorithm: on receiving a marker on all inputs, snapshot state, then forward markers on all outputs.
3. Workers write snapshots to shared storage and report to the coordinator.
4. Commit/abort same as barrier-based.

### 6.4 Message contract (sketch)

| Message | Direction | Content |
|---------|-----------|---------|
| `CheckpointRequest` | Coordinator → Worker | `{ checkpoint_id, barrier_t? }` |
| `CheckpointDone` | Worker → Coordinator | `{ checkpoint_id, shard_id, success }` |
| `CheckpointCommitted` | Coordinator → Worker | `{ checkpoint_id }` (optional ack) |

### 6.5 Storage layout (distributed)

```
<base>/
  <checkpoint_id>/
    COMMITTED          # written when all workers reported
    metadata.json      # global metadata (barrier T, worker list)
    shard_0/           # worker 0 snapshots
      metadata.json
      <node_id>.bin
    shard_1/
      ...
```

---

## 7. Recovery from worker failure (phase 5)

When a worker fails, the coordinator (or orchestration layer) recovers using the last committed checkpoint.

### 7.1 Detection

- **Health checks:** Coordinator periodically pings workers; missing response → worker presumed dead.
- **Explicit failure:** Worker sends a failure report before exiting (optional).
- **Orchestrator:** Kubernetes, systemd, or similar may signal the worker is down.

### 7.2 Recovery steps

1. **Stop routing** to the failed worker; do not send new input for its keys.
2. **Choose checkpoint:** Use the last **committed** checkpoint (not partial or in-progress).
3. **Reassign keys:** The failed worker’s key range (e.g. `shard_id`, `total_shards` before failure) must be reassigned:
   - **Option A:** Reduce `total_shards` by 1; remaining workers absorb the keys (e.g. `hash(key) % (N-1)`).
   - **Option B:** Start a new worker (replace); assign it the failed worker’s `shard_id` and key range.
4. **Restore:** For each remaining (or new) worker:
   - Load checkpoint from shared storage for its shard(s).
   - Call `restore_from_checkpoint` (or equivalent) to load state into the graph.
   - If a worker gains keys (Option A), use `import_state_for_keys` with state exported from the checkpoint for those keys (see [cluster-sharding.md](cluster-sharding.md) state migration).
5. **Resume:** Workers resume from the checkpoint position; sources replay input from recorded positions. Exactly-once state ensures no double-apply.

### 7.3 Key reassignment and state migration

- If **Option B** (replace worker): New worker gets same `shard_id`; restore from `shard_<id>/` in the checkpoint. No state migration between workers.
- If **Option A** (absorb keys): Workers that gain keys must import state. The checkpoint contains per-shard snapshots; the coordinator (or a migration step) extracts state for the moved keys from the failed shard’s checkpoint and provides it to the gaining workers. Use `Graph::import_state_for_keys` on the gaining workers.

### 7.4 Input replay

- **Positions:** Checkpoint metadata records input positions (e.g. Kafka offsets, logical time) per shard.
- **Replay:** After restore, each worker resumes consuming from its recorded position. Sources (Kafka, etc.) must support seek to that position.
- **Idempotency:** Exactly-once state and idempotent sinks ensure replay does not produce duplicates.

---

## 8. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | **Local checkpointing:** State backends support `snapshot`/`restore`. Graph (or execution) can trigger “checkpoint now” (quiesce, snapshot all, record positions). Document format and storage. **Done:** `checkpoint::CheckpointStorage`, `FileCheckpointStorage`, `CheckpointId`, `CheckpointMetadata` in `src/checkpoint.rs`. Format: `<base>/<id>/metadata.json` and `<base>/<id>/<node_id>.bin`. |
| **2** | **Restore and resume:** On startup, `restore_from_checkpoint(id)`; load state, set positions; sources replay from positions. **Done:** `Graph::restore_from_checkpoint(storage, id)` loads checkpoint, calls `Node::restore_state` on nodes with snapshot data, stores position; `restored_position()` for query; time counter initialized from position when executing with progress. `Node::restore_state` default no-op; stateful nodes override. Integrate with exactly-once state. |
| **3** | **Periodic checkpointing:** Timer or “every N items” triggers checkpoint in the background (or at safe points). **Done:** `Graph::trigger_checkpoint(storage)` snapshots all stateful nodes via `Node::snapshot_state`, saves metadata and snapshots to storage, returns `CheckpointId`. Sequenced IDs via `checkpoint_sequence`. Tests: `test_trigger_checkpoint`, `test_trigger_checkpoint_no_nodes_fails`. |
| **4** | (With distribution) **Coordinated protocol:** Coordinator sends barrier or markers; workers snapshot and report; commit when all done. **Documented:** §6 (barrier-based, Chandy–Lamport, message contract, storage layout). |
| **5** | **Recovery from failure:** On worker failure, coordinator (or remaining workers) restore from last committed checkpoint; reassign failed worker’s keys; resume. |

---

## 9. References

- Gap analysis §15 (Distributed checkpointing).
- [exactly-once-state.md](exactly-once-state.md), [cluster-sharding.md](cluster-sharding.md).
- Chandy–Lamport snapshot algorithm; Flink/Kafka Streams checkpointing.
