# Auto-Scaling Clusters

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §16.

**Dependencies:** Mature cluster sharding, Production-hardened cluster tooling.

**Status:** Deferred until cluster sharding exists. Auto-scaling (add/remove workers based on load) is a future goal. It will require cluster sharding, metrics (see [production-cluster-tooling.md](production-cluster-tooling.md)), and a rebalance API. Use Kubernetes HPA or a custom controller that drives rebalance based on metrics when distribution is available.

**Implementation status:** See [IMPLEMENTATION-STATUS.md](IMPLEMENTATION-STATUS.md#auto-scaling-clustersmd). Phase 1 (document as future goal) done. Phases 2–4 not started; explicitly deferred.

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
| **2** | Expose metrics and rebalance API so that external controllers can scale and rebalance. **Done:** `record_items_in`/`record_items_out`, `record_shard_assignment`; `InMemoryCoordinator::scale_to`; see §5.1. |
| **3** | (Optional) Provide an example or reference design for Kubernetes HPA + rebalance. **Done:** §5.2 (Deployment, HPA YAML). |
| **4** | (Optional) Implement a simple built-in scaler that reads metrics and calls rebalance (e.g. “scale to N workers when backlog > threshold”). **Done:** `scaler` module (§5.3). |

### 5.1 Phase 2 implementation details

**Metrics for scaling (Prometheus):**

- `streamweave_items_in_total`, `streamweave_items_out_total` – counters; derive items/sec for throughput-based scaling.
- `streamweave_shard_id`, `streamweave_total_shards` – gauges; recorded when `Graph` has `ShardConfig` (set automatically in `run_dataflow`).
- Call `metrics::record_items_in` / `record_items_out` from instrumented nodes for throughput.

**Rebalance API:**

- `RebalanceCoordinator::current_assignment()` – observe current shard assignment.
- `InMemoryCoordinator::scale_to(total_shards)` – set cluster size (tests/demos).
- For production: implement `RebalanceCoordinator` with etcd/ZooKeeper/Kafka; controller updates the shared store.

### 5.2 Kubernetes HPA + rebalance example (Phase 3)

Reference design for scaling StreamWeave workers with Kubernetes HPA:

**1. Deployment** – Each pod runs one graph instance with `SHARD_ID` and `TOTAL_SHARDS` from env (from Downward API or a controller):

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: streamweave-workers
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: worker
        image: streamweave:latest
        env:
        - name: SHARD_ID
          valueFrom:
            fieldRef:
              fieldPath: metadata.annotations['streamweave.io/shard-id']
        - name: TOTAL_SHARDS
          value: "3"
        ports:
        - name: metrics
          containerPort: 9090
```

**2. HPA** – Scale based on CPU or custom metric (e.g. `streamweave_items_in_total` rate from Prometheus):

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: streamweave-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: streamweave-workers
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

**3. Rebalance** – When HPA changes replicas, a sidecar, init container, or operator must update the coordinator (etcd/Kafka) with the new `total_shards`, then trigger drain/migrate/resume. Use `RebalanceCoordinator` with your coordination backend.

### 5.3 Built-in scaler (Phase 4)

The `streamweave::scaler` module provides a standards-oriented built-in scaler:

- **Config:** `ScalerConfig` – min/max shards, scale-up/scale-down thresholds (throughput and/or backlog), stabilization windows, cooldown. Use `ScalerConfig::validate()` and `ScalerConfig::clamp_shards()`.
- **Policy:** `decide` – pure function: given config, current shards, metrics, and stabilization booleans, returns `ScalerDecision` (target shards and optional `ScaleReason`). Caller tracks how long conditions have been satisfied.
- **Loop:** `run_one_tick` – stateful tick: takes `ScalerState`, current shards, `TickMetrics` (items/sec, backlog), and current time; returns updated decision and state. Caller obtains `current_shards` from `RebalanceCoordinator::current_assignment().await.total_shards`, then if `decision.reason.is_some()` calls `InMemoryCoordinator::scale_to(decision.target_shards)` (or equivalent).
- **Observability:** `record_scale_decision` – call after applying a scale; logs and records `streamweave_scaler_scale_total` (counter by reason) and `streamweave_scaler_target_shards` (gauge).

**Single decision maker:** Use either the built-in scaler or an external controller (e.g. HPA) for scaling, not both. Disable the built-in scaler by not running the tick loop.

**Example loop (pseudocode):** every N seconds: get assignment from coordinator; get items_per_sec and backlog from Prometheus or in-memory counters; `let (decision, new_state) = run_one_tick(config, state, assignment.total_shards, metrics, now)`; if `decision.reason.is_some()` { coordinator.scale_to(decision.target_shards); record_scale_decision(decision.reason.unwrap(), current, decision.target_shards); }; state = new_state.

---

## 6. References

- Gap analysis §16 (Auto-scaling clusters).
- [cluster-sharding.md](cluster-sharding.md), [production-cluster-tooling.md](production-cluster-tooling.md).
- Kubernetes HPA/VPA; custom metrics and Prometheus.
