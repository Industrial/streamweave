# Mature Cluster Sharding

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §14.

**Dependencies:** Exactly-once state (for correct rebalance and state migration).

**Implementation status:** See [IMPLEMENTATION-STATUS.md](IMPLEMENTATION-STATUS.md#cluster-shardingmd). Phases 1–5 done. Phase 5: `rebalance` module (ShardAssignment, MigrationPlan, compute_migration_plan, RebalanceCoordinator trait, InMemoryCoordinator); worker protocol documented in §8.

---

## 1. Objective and rationale

**Objective:** Partition the logical graph (or data) across **multiple workers** so that each worker runs a subset of state and computation. **Routing** is by key or partition; **state** is local to the shard that owns the key. “Mature” means stable partitioning, rebalancing, and state migration.

**Why it matters:**

- Horizontal scaling and locality: large state and high throughput require sharding.
- Single-process StreamWeave cannot scale beyond one machine; sharding is the first step toward a distributed runtime.

---

## 2. Current state in StreamWeave

- **No sharding:** One graph, one process.
- **No distribution:** No workers, no partition key, no routing layer.
- **State:** In-memory per process; no notion of “shard” or “key ownership.”

---

## 3. Design principles

### 3.1 Partitioning contract

Define in the API how **partitioning** works:

- **Partition key:** A subset of the data (e.g. `user_id`, `(tenant_id, session_id)`) that determines which shard handles the record.
- **Contract:** “This node’s input is partitioned by key K.” The same logical graph is instantiated **N** times (one per shard); each instance sees only the subset of data whose key hashes (or is assigned) to that shard.
- **Routing:** A **driver** or **ingress** layer hashes the key and sends each record to the correct shard (process or thread).

### 3.2 State locality

- **Stateful nodes:** State for key K lives only on the shard that owns K. So aggregations, joins, and windows are key-scoped and local to one shard.
- **Exactly-once:** When rebalancing (adding/removing workers), state must be **migrated** without loss or duplication. This requires exactly-once state and a clear ownership transfer protocol (see [exactly-once-state.md](exactly-once-state.md)).

### 3.3 Worker model

- **Worker:** One process (or one thread) that runs one **instance** of the graph (or a subgraph) for a **set of partition keys** (e.g. shard 0 owns keys 0..99, shard 1 owns 100..199, or consistent hashing).
- **Driver:** Optional central or distributed component that assigns keys to workers, routes input, and collects output (or output is written to a shared sink, e.g. Kafka).

---

## 4. Detailed design

### 4.1 First: partitioning contract in the API

- **Declare partition key:** e.g. “this graph (or this input port) is partitioned by key extracted from the payload.” The key extractor is a function `Payload -> PartitionKey` (or a key type and trait).
- **Same graph, N instances:** The user defines the graph once; the runtime (or the user) instantiates it N times. Each instance gets a **shard id** and a **key range** (or consistent-hash range). Input is filtered or routed so that each instance only sees keys in its range.
- **External systems:** Kafka, etc., can provide partitioning (e.g. Kafka partition = shard). StreamWeave can consume from N Kafka partitions and run N graph instances, one per partition. So “sharding” can be “run N processes, each consuming one partition”; the “mature” part is when StreamWeave itself manages the partition assignment and rebalancing.

### 4.2 Second: distribution layer

- **Deploy N graph instances:** A **driver** (or script) deploys N processes (or N tasks), each running the same graph with different `shard_id` and key range.
- **Input routing:** Records are sent to the appropriate instance (e.g. by hash(key) % N, or by a lookup table). This can be done by an external load balancer, Kafka partition, or a small “router” service that forwards to workers.
- **Output:** Each instance produces output; outputs can be merged (e.g. to a Kafka topic, or to a central collector). Order is only preserved per key (per shard).

### 4.3 Mature: rebalancing and state migration

When **adding or removing workers**:

- **Reassignment:** Key ranges are reassigned (e.g. shard 0 used to own 0..99, now owns 0..49; new shard 2 owns 50..99). A coordinator (or leader) decides the new assignment.
- **State migration:** Shards that **lose** keys must **send** their state for those keys to the shards that **gain** them. Shards that **gain** keys must **load** that state and then accept new data for those keys. During migration, no duplicate or lost updates (exactly-once state).
- **Drain and handoff:** Optionally, drain the old shard (stop accepting new data for moved keys), serialize state, send to new shard, new shard loads and resumes. This requires a **pause** or **drain** protocol so that in-flight data is completed before handoff.
- **Consistent hashing:** Alternative to range assignment; reduces the amount of state that moves when adding/removing one worker (only adjacent keys move).

### 4.4 What StreamWeave must provide (eventually)

- **Partition key extraction:** Configurable per graph or per input (e.g. “partition by field X”).
- **Shard identity:** Each instance knows its `shard_id` and key range (or assignment).
- **State snapshot and restore:** So that state can be serialized and sent to another shard (see [exactly-once-state.md](exactly-once-state.md), [distributed-checkpointing.md](distributed-checkpointing.md)).
- **Coordination (optional):** If StreamWeave runs the coordinator, it needs membership (who are the workers), assignment (which keys per worker), and rebalance protocol. Otherwise, the user can use external systems (e.g. Kafka consumer group rebalance) and pass assignment into StreamWeave.

### 4.5 What to avoid

- Building a full distributed runtime (RPC, service discovery, cluster membership) from scratch. Prefer reusing existing infra (Kafka, Kubernetes, etc.) and defining clear boundaries: “StreamWeave runs in one process per shard; partitioning and routing are external or a thin layer.”

---

## 5. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | Define **partitioning contract** in the API (key extractor, “this graph is partitioned by key”). Document “run N instances with different key ranges” as a user-driven pattern. **Done:** `PartitionKey`, `PartitionKeyExtractor`, `PartitioningConfig`, `partition_by_key()` in `src/partitioning.rs`. See §6. |
| **2** | Add **shard_id** (and optional key range) to graph or execution config so that nodes can behave differently per shard (e.g. state path). **Done:** `ShardConfig`, `Graph::set_shard_config()`, `Graph::shard_id()`, `Graph::total_shards()`, `GraphBuilder::shard_config()`. Key range derived from `(shard_id, total_shards)`. |
| **3** | Implement or document a **distribution layer**: driver that deploys N processes, routes input by key, merges output. Can be minimal (script + env vars). **Done:** §7 documents script + env vars, input routing (Kafka, router, filter), output merging, Kubernetes/systemd. |
| **4** | **State migration:** With exactly-once state and checkpointing, implement “export state for keys K” and “import state for keys K” so that a coordinator can move state between shards on rebalance. **Done:** `ExactlyOnceStateBackend::snapshot_for_keys`, `restore_keys`; `Node::export_state_for_keys`, `import_state_for_keys`; `Graph::export_state_for_keys`, `import_state_for_keys`. |
| **5** | **Rebalance protocol:** Define how assignment changes are communicated and how workers drain, migrate state, and resume. **Done:** `rebalance` module, `RebalanceCoordinator` trait, `InMemoryCoordinator`, `compute_migration_plan`, worker protocol in §8. |

---

## 6. Run N instances (user-driven pattern)

With the partitioning contract in place, you can run N graph instances manually:

1. **Define the graph** once (with nodes that are keyed/partitionable).
2. **Create a partition key extractor** using `streamweave::partitioning::partition_by_key()`.
3. **Run N processes** (e.g. via a script or systemd), each with:
   - `SHARD_ID=0..N-1` (or similar)
   - Input routed so that each process receives only records whose key hashes to its shard (e.g. `hash(key) % N == shard_id`).

**Example:** Kafka consumer group with N partitions. Run N StreamWeave processes; each consumes one Kafka partition. Kafka provides the partitioning; each process runs the same graph. No StreamWeave-internal sharding yet—partitioning is external.

**With phase 2 (ShardConfig):** The graph can call `graph.shard_config()` to get `ShardConfig`, then use `owns_key(key)` to check "do I own this key?" and reject or route records accordingly. Use `GraphBuilder::shard_config(shard_id, total_shards)` or `Graph::set_shard_config()` when creating the graph.

---

## 7. Distribution layer (phase 3)

A **minimal distribution layer** deploys N graph instances, routes input by key, and merges output. StreamWeave does not provide a built-in driver; use scripts, Kubernetes, or systemd.

### 7.1 Deploying N processes (script + env vars)

Each process reads `SHARD_ID` and `TOTAL_SHARDS` from the environment and passes them to the graph:

```bash
#!/bin/bash
# run-sharded.sh - Deploy N StreamWeave instances (one per shard)
TOTAL_SHARDS=${TOTAL_SHARDS:-4}
for i in $(seq 0 $((TOTAL_SHARDS - 1))); do
  SHARD_ID=$i TOTAL_SHARDS=$TOTAL_SHARDS ./your-streamweave-app &
done
wait
```

In your app, read env and configure the graph:

```rust
let shard_id = std::env::var("SHARD_ID").ok().and_then(|s| s.parse().ok());
let total_shards = std::env::var("TOTAL_SHARDS").ok().and_then(|s| s.parse().ok());
let mut builder = GraphBuilder::new("my_graph")...;
if let (Some(sid), Some(ts)) = (shard_id, total_shards) {
  builder = builder.shard_config(sid, ts);
}
let graph = builder.build()?;
```

### 7.2 Input routing

**Option A – Kafka:** Create a topic with N partitions. Run N processes; each consumes from partition `shard_id`. Kafka provides routing; no extra code.

**Option B – Custom router:** A separate service receives all input, extracts the partition key, hashes it, and forwards to the correct worker (e.g. over HTTP or a queue). Workers listen on distinct ports.

**Option C – Shared queue with filter:** All workers consume from one queue; each filters to `owns_key(key)` and drops records it doesn't own. Less efficient but simple.

### 7.3 Output merging

**Option A – Kafka sink:** Each process writes to the same Kafka topic. Order is per-key (per shard). Downstream consumers merge automatically.

**Option B – Shared storage:** Each process writes to a path like `output/shard_{id}/` or appends to a shared file/DB with a shard identifier.

**Option C – Central collector:** Workers send output to a collector service that merges and forwards.

### 7.4 Kubernetes / systemd

- **Kubernetes:** Run N replicas with `SHARD_ID` from a downward API or ConfigMap (`shard-0` … `shard-N`).
- **systemd:** N template units `streamweave@0.service` … `streamweave@N.service`, each with `Environment="SHARD_ID=%i"` and `Environment="TOTAL_SHARDS=N"`.

---

## 8. Rebalance protocol (phase 5)

When workers join or leave, the coordinator decides the new assignment. StreamWeave provides types and a coordinator trait; the actual coordination (e.g. Kafka consumer group, ZooKeeper) is external.

### 8.1 Types

- **`ShardAssignment`** (`rebalance::ShardAssignment`): `(shard_id, total_shards)`. Use `owns_key(key)` and `shard_for_key(key)`.
- **`MigrationPlan`**: For a shard, `keys_to_export` (state to send) and `keys_to_import` (state to receive).
- **`compute_migration_plan(shard_id, old_total, new_total, keys)`**: Computes export/import sets when `total_shards` changes.
- **`RebalanceCoordinator`** trait: `current_assignment()` and `await_assignment_change()`. Implement with your coordination system.
- **`InMemoryCoordinator`**: For tests; `add_worker()`, `remove_worker()` simulate cluster changes.

### 8.2 Worker protocol (drain / migrate / resume)

1. **Detect change**: Coordinator reports new assignment (e.g. `total_shards` increased).
2. **Compute plan**: `compute_migration_plan(my_shard_id, old_total, new_total, Some(&known_keys))`.
3. **Drain** (losing keys): `graph.pause()` → for each stateful node, `graph.export_state_for_keys(node_id, &plan.keys_to_export)` → send bytes to coordinator or gaining shard.
4. **Receive** (gaining keys): Receive state bytes from coordinator → for each stateful node, `graph.import_state_for_keys(node_id, data)`.
5. **Resume**: `graph.set_shard_config(my_shard_id, new_total)` → `graph.resume()`.

The graph already provides `pause`, `resume`, `export_state_for_keys`, `import_state_for_keys`, `set_shard_config`. Transport of state bytes between processes is the user’s responsibility (e.g. HTTP, gRPC, shared storage).

---

## 9. References

- Gap analysis §14 (Mature cluster sharding).
- [exactly-once-state.md](exactly-once-state.md), [distributed-checkpointing.md](distributed-checkpointing.md).
- Kafka consumer groups: partition assignment and rebalance as inspiration.
- [scope-in-process-no-distributed-fault-tolerance.md](scope-in-process-no-distributed-fault-tolerance.md) for current scope.
