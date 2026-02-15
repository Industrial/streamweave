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

## 6. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | **Local checkpointing:** State backends support `snapshot`/`restore`. Graph (or execution) can trigger “checkpoint now” (quiesce, snapshot all, record positions). Document format and storage. |
| **2** | **Restore and resume:** On startup, `restore_from_checkpoint(id)`; load state, set positions; sources replay from positions. Integrate with exactly-once state. |
| **3** | **Periodic checkpointing:** Timer or “every N items” triggers checkpoint in the background (or at safe points). |
| **4** | (With distribution) **Coordinated protocol:** Coordinator sends barrier or markers; workers snapshot and report; commit when all done. |
| **5** | **Recovery from failure:** On worker failure, coordinator (or remaining workers) restore from last committed checkpoint; reassign failed worker’s keys; resume. |

---

## 7. References

- Gap analysis §15 (Distributed checkpointing).
- [exactly-once-state.md](exactly-once-state.md), [cluster-sharding.md](cluster-sharding.md).
- Chandy–Lamport snapshot algorithm; Flink/Kafka Streams checkpointing.
