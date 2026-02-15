# Auto-Scaling Clusters

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §16.

**Dependencies:** Mature cluster sharding, Production-hardened cluster tooling.

**Status:** Deferred until cluster sharding exists. Auto-scaling (add/remove workers based on load) is a future goal. It will require cluster sharding, metrics (see [production-cluster-tooling.md](production-cluster-tooling.md)), and a rebalance API. Use Kubernetes HPA or a custom controller that drives rebalance based on metrics when distribution is available.

---

## 1. Objective and rationale

**Objective:** The runtime (or an external controller) **adds or removes workers** (or replicas) based on load, backlog, or policy, and **rebalances** work (partitions, shards) across the new set. This provides operational flexibility and cost efficiency in cloud environments.

**Why it matters:**

- Not required for correctness, but expected in “production” cluster tooling.
- Enables scaling out under load and scaling in when idle; reduces cost and improves latency when demand varies.

---

## 2. Current state in StreamWeave

- **No clusters:** Single process; no workers, no scaling.
- **No auto-scaling:** No controller that adds/removes nodes based on metrics.
- **Defer:** This capability is **low priority** until a distributed runtime (workers, sharding, state placement) exists. The gap analysis recommends documenting it as a **future goal**.

---

## 3. What auto-scaling entails

### 3.1 Metrics

- **Throughput:** Items per second (per shard or globally). Scale out when throughput is high; scale in when low.
- **Backlog / lag:** e.g. Kafka consumer lag, or “number of items waiting in queues.” Scale out when lag exceeds a threshold.
- **Resource utilization:** CPU, memory per worker. Scale out when utilization is high; scale in when low for a sustained period.
- **Custom:** User-defined metrics (e.g. “p99 latency”) can drive scaling policies.

### 3.2 Scaling policy

- **Scale out:** Add one or more workers; assign new shards (or split existing key ranges) to the new workers; migrate state for moved keys (see [cluster-sharding.md](cluster-sharding.md)); start consumption.
- **Scale in:** Remove one or more workers; reassign their shards to remaining workers; migrate state; drain and stop the removed workers.
- **Cooldown / hysteresis:** Avoid flapping (e.g. scale out then immediately scale in). Use cooldown periods and thresholds (e.g. “scale in only if utilization < 30% for 5 minutes”).

### 3.3 Integration with orchestrators

- **Kubernetes:** Use **Horizontal Pod Autoscaler (HPA)** or **Vertical Pod Autoscaler (VPA)** based on CPU/memory or custom metrics (e.g. Prometheus). StreamWeave exposes metrics (see [production-cluster-tooling.md](production-cluster-tooling.md)); the orchestrator scales the number of pods (workers). **Rebalancing** may require a custom controller that (a) observes new pod count, (b) triggers partition reassignment and state migration inside StreamWeave (or via a coordinator), (c) drains old pods.
- **Custom controller:** A small service that watches metrics and the current worker set, decides to add/remove workers, and calls StreamWeave’s (or the cluster’s) rebalance API. StreamWeave does not need to implement the scaling logic itself; it only needs to support “add worker,” “remove worker,” and “rebalance” operations that the controller can invoke.
- **Avoid:** Building a full orchestrator inside StreamWeave. Prefer integration with Kubernetes, Nomad, or similar.

### 3.4 Rebalance and state migration

- Adding/removing workers implies **rebalancing** shards and **state migration** (see [cluster-sharding.md](cluster-sharding.md)). Auto-scaling must trigger the same rebalance protocol; the only difference is that the trigger is “policy decided to add/remove a worker” rather than “user requested” or “worker failed.”
- **Graceful scale-in:** When scaling in, drain the worker (stop accepting new keys), migrate state for its keys to remaining workers, then terminate. This can take time; the scaling controller should wait or use a timeout.

---

## 4. Detailed design (future)

### 4.1 StreamWeave’s role

- **Expose metrics:** Throughput, lag, utilization per worker (see [production-cluster-tooling.md](production-cluster-tooling.md)). So that an external scaler can make decisions.
- **Expose rebalance API:** “Add worker W with assignment A,” “Remove worker W” (trigger drain and migration), “Current assignment.” The coordinator (if inside StreamWeave) or the deployment layer (if external) uses this.
- **Do not implement:** The actual “when to scale” policy (e.g. “scale out when CPU > 80%”). That belongs in the orchestrator or a custom controller.

### 4.2 Example: Kubernetes HPA

- StreamWeave runs as a Deployment with N replicas (N = number of shards or workers).
- Each pod runs one graph instance with a shard_id (from env or config).
- HPA scales replicas based on CPU or custom metric (e.g. `streamweave_backlog_size` from Prometheus).
- When replicas change, a **sidecar** or **init container** or **operator** runs the rebalance: reassigns key ranges to the new set of pods, triggers state migration, and updates each pod’s config (shard_id, key range). StreamWeave must support “reconfigure my key range” and “export/import state” for this to work.
- Alternatively, each replica is statically assigned a fixed shard (e.g. replica 0 = shard 0); then scaling only adds/removes replicas for a fixed set of shards (e.g. scale from 3 to 6 by doubling each shard’s replicas for redundancy). Rebalance is simpler (no key movement) but scaling granularity is different.

### 4.3 Documentation (now)

- **Document as future goal:** “Auto-scaling (add/remove workers based on load) is planned. It will require cluster sharding, metrics, and rebalance support. We recommend Kubernetes HPA or a custom controller that drives rebalance based on metrics.”
- **No implementation** until distribution and rebalance are in place.

---

## 5. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | Document auto-scaling as a future goal; document that it depends on sharding and production tooling (metrics, rebalance API). **Done:** Status callout and §4.3. |
| **2** | (With distribution) Expose metrics and rebalance API so that external controllers can scale and rebalance. |
| **3** | (Optional) Provide an example or reference design for Kubernetes HPA + rebalance (e.g. operator or Helm chart with hooks). |
| **4** | (Optional) Implement a simple built-in scaler that reads metrics and calls rebalance (e.g. “scale to N workers when backlog > threshold”). Defer until core distribution is stable. |

---

## 6. References

- Gap analysis §16 (Auto-scaling clusters).
- [cluster-sharding.md](cluster-sharding.md), [production-cluster-tooling.md](production-cluster-tooling.md).
- Kubernetes HPA/VPA; custom metrics and Prometheus.
