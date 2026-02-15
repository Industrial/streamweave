# Mature Cluster Sharding

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §14.

**Dependencies:** Exactly-once state (for correct rebalance and state migration).

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
| **1** | Define **partitioning contract** in the API (key extractor, “this graph is partitioned by key”). Document “run N instances with different key ranges” as a user-driven pattern. |
| **2** | Add **shard_id** (and optional key range) to graph or execution config so that nodes can behave differently per shard (e.g. state path). |
| **3** | Implement or document a **distribution layer**: driver that deploys N processes, routes input by key, merges output. Can be minimal (script + env vars). |
| **4** | **State migration:** With exactly-once state and checkpointing, implement “export state for keys K” and “import state for keys K” so that a coordinator can move state between shards on rebalance. |
| **5** | **Rebalance protocol:** Define how assignment changes are communicated and how workers drain, migrate state, and resume. |

---

## 6. References

- Gap analysis §14 (Mature cluster sharding).
- [exactly-once-state.md](exactly-once-state.md), [distributed-checkpointing.md](distributed-checkpointing.md).
- Kafka consumer groups: partition assignment and rebalance as inspiration.
- [scope-in-process-no-distributed-fault-tolerance.md](scope-in-process-no-distributed-fault-tolerance.md) for current scope.
