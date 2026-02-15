# Implementation Status

This document summarizes what has been implemented vs. what remains for each capability described in the gap analysis and design docs. **Last updated:** 2025-02-15.

---

## Quick reference

| Capability | Status | Document |
|------------|--------|----------|
| Logical timestamps | ✅ Complete | [logical-timestamps-timely-and-streamweave.md](logical-timestamps-timely-and-streamweave.md) |
| Event-time semantics | ✅ Complete | [event-time-semantics.md](event-time-semantics.md) |
| Progress tracking (watermarks) | ✅ Complete | [progress-tracking.md](progress-tracking.md) |
| Deterministic execution | ✅ Complete | [deterministic-execution.md](deterministic-execution.md) |
| Exactly-once state | ✅ Complete | [exactly-once-state.md](exactly-once-state.md) |
| Scope (in-process) | ✅ Complete | [scope-in-process-no-distributed-fault-tolerance.md](scope-in-process-no-distributed-fault-tolerance.md) |
| Windowing | ✅ Complete | [windowing.md](windowing.md) |
| Event-time processing | ✅ Complete | [event-time-processing.md](event-time-processing.md) |
| Timestamped differential dataflow | ✅ Complete | [timestamped-differential-dataflow.md](timestamped-differential-dataflow.md) |
| Cyclic iterative dataflows | ✅ Complete | [cyclic-iterative-dataflows.md](cyclic-iterative-dataflows.md) |
| Local checkpointing | ✅ Complete | [distributed-checkpointing.md](distributed-checkpointing.md) |
| Cluster sharding | ✅ Complete | [cluster-sharding.md](cluster-sharding.md) |
| Production tooling | ✅ Complete | [production-cluster-tooling.md](production-cluster-tooling.md) |
| Actor supervision trees | ⚠️ Partial | [actor-supervision-trees.md](actor-supervision-trees.md) |
| Incremental recomputation | ⚠️ Partial | [incremental-recomputation.md](incremental-recomputation.md) |
| Distributed checkpointing | ❌ Not started | [distributed-checkpointing.md](distributed-checkpointing.md) |
| Auto-scaling clusters | ❌ Deferred | [auto-scaling-clusters.md](auto-scaling-clusters.md) |

---

## Detailed status by document

### actor-supervision-trees.md
**Status: ⚠️ Partial**

| Phase | Status | Notes |
|-------|--------|-------|
| 1 – SupervisionPolicy, FailureAction | ✅ Done | `src/supervision.rs` |
| 2 – Failure reporting | ✅ Done | `FailureReport`, `enable_failure_reporting()`, `wait_for_completion` |
| 3 – Supervisor loop (Restart) | ✅ Done | `execute_with_supervision()`, graph-level restart |
| 4 – RestartGroup, Escalate (distinct) | ✅ Done | `set_supervision_group()`, RestartGroup shared count, Restart per-node, Stop vs Escalate distinct |
| 5 – Tests | ✅ Done | `test_execute_with_supervision_*` |

**Limitations:** Flat hierarchy (graph supervises all nodes). No subgraph-as-supervision-unit.

---

### auto-scaling-clusters.md
**Status: ⚠️ Partial (Phase 2 done)**

| Phase | Status | Notes |
|-------|--------|-------|
| 1 – Document as future goal | ✅ Done | Doc states deferred until sharding exists |
| 2 – Metrics + rebalance API | ✅ Done | `record_items_in`/`record_items_out`, `record_shard_assignment`; `InMemoryCoordinator::scale_to`; RebalanceCoordinator trait |
| 3 – Kubernetes HPA example | ✅ Done | Deployment + HPA YAML in auto-scaling-clusters.md §5.2 |
| 4 – Built-in scaler | ⏳ Not started | Deferred |

**Note:** Phase 2 enables external controllers to observe metrics and trigger rebalance via coordinator.

---

### cluster-sharding.md
**Status: ⚠️ Partial (API complete, runtime not)**

| Phase | Status | Notes |
|-------|--------|-------|
| 1 – Partitioning contract | ✅ Done | `PartitionKey`, `PartitionKeyExtractor`, `partition_by_key()` in `src/partitioning.rs` |
| 2 – ShardConfig | ✅ Done | `ShardConfig`, `set_shard_config()`, `owns_key()`, `shard_id()`, `total_shards()` |
| 3 – Distribution layer | ✅ Documented | Script + env vars, Kafka routing, Kubernetes/systemd patterns |
| 4 – State migration | ✅ Done | `export_state_for_keys`, `import_state_for_keys` on Node and Graph |
| 5 – Rebalance protocol | ✅ Done | `rebalance` module: ShardAssignment, MigrationPlan, compute_migration_plan, RebalanceCoordinator trait, InMemoryCoordinator; worker protocol (drain/migrate/resume) documented in cluster-sharding.md §8 |

**Note:** Coordinator is a trait; use external systems (Kafka, etc.) or InMemoryCoordinator for tests.

---

### cyclic-iterative-dataflows.md
**Status: ✅ Complete**

| Phase | Status | Notes |
|-------|--------|-------|
| Option A – BoundedIterationNode | ✅ Done | `src/nodes/bounded_iteration_node.rs` |
| Option B – Timely-style rounds | ✅ Done | `Graph::execute_with_rounds`, `has_cycles`, `feedback_edges` in `src/graph.rs` |

---

### deterministic-execution.md
**Status: ✅ Complete**

| Phase | Status | Notes |
|-------|--------|-------|
| ExecutionMode::Deterministic | ✅ Done | `set_execution_mode()`, topological order |
| execute_deterministic | ✅ Done | Deterministic merge, MergeNode::new_deterministic |
| Tests | ✅ Done | `test_*_deterministic` |

---

### distributed-checkpointing.md
**Status: ⚠️ Partial (local only)**

| Phase | Status | Notes |
|-------|--------|-------|
| 1 – Local checkpointing | ✅ Done | `CheckpointStorage`, `FileCheckpointStorage`, `trigger_checkpoint`, `restore_from_checkpoint` |
| 2 – Restore and resume | ✅ Done | `Node::restore_state`, `restored_position()` |
| 3 – Periodic trigger | ✅ Done | `trigger_checkpoint()` (caller-driven; no built-in timer) |
| 4 – Coordinated protocol | ✅ Done | CheckpointCoordinator, DistributedCheckpointStorage, InMemoryCheckpointCoordinator; trigger_checkpoint_for_coordination, restore_from_distributed_checkpoint |
| 5 – Recovery from failure | ✅ Done | compute_recovery_plan_absorb, RecoveryStep; restore + import_state_for_keys per §7 |

**Note:** Coordinated distributed checkpointing (barrier-based, Chandy–Lamport) is documented as a specification for future work.

---

### event-time-processing.md
**Status: ✅ Complete**

| Component | Status | Notes |
|-----------|--------|-------|
| Event-time semantics | ✅ Done | HasEventTime, EventTimeExtractorNode |
| Progress/watermarks | ✅ Done | WatermarkInjectorNode, progress handle |
| Event-time windows | ✅ Done | Tumbling, sliding, session |
| Late-data policy | ✅ Done | Drop, SideOutput (LateDataPolicy) |
| Tests | ✅ Done | Out-of-order, late data tests |

---

### event-time-semantics.md
**Status: ✅ Complete**

| Component | Status | Notes |
|-----------|--------|-------|
| HasEventTime trait | ✅ Done | `src/time.rs` |
| EventTimeExtractorNode | ✅ Done | Extract event time from payloads |
| connect_timestamped_input_channel | ✅ Done | User-provided event time as logical time |

---

### exactly-once-state.md
**Status: ✅ Complete**

| Phase | Status | Notes |
|-------|--------|-------|
| ExactlyOnceStateBackend trait | ✅ Done | `src/state.rs` |
| HashMapStateBackend | ✅ Done | put/get/snapshot/restore |
| KeyedStateBackend | ✅ Done | Key-scoped state |
| StatefulNodeDriver | ✅ Done | Trait for stateful nodes |
| Idempotent sink pattern | ✅ Documented | docs/exactly-once-state.md |

---

### incremental-recomputation.md
**Status: ⚠️ Partial (Phase 4 implemented)**

| Level | Status | Notes |
|-------|--------|-------|
| Short term – patterns doc | ✅ Done | Documented in doc |
| MemoizingMapNode | ✅ Done | `src/nodes/memoizing_map_node.rs` |
| Replay from checkpoint | ✅ Done | Via restore_from_checkpoint + exactly-once state |
| Medium term – differential operators | ✅ Done | DifferentialGroupByNode, DifferentialJoinNode are incremental by construction |
| Long term – time-range recomputation | ⚠️ Partial | `TimeRange`, `RecomputeRequest`, `RecomputePlan`; `plan_recompute`; `ProgressHandle::from_sink_frontiers`, `sink_frontiers()`; `Graph::plan_recompute`, `Graph::execute_recompute` (runs full graph); subgraph time-scoped execution deferred |

---

### logical-timestamps-timely-and-streamweave.md
**Status: ✅ Complete**

| Component | Status | Notes |
|-----------|--------|-------|
| LogicalTime | ✅ Done | `src/time.rs` |
| Timestamped<T> | ✅ Done | Envelope for (payload, time) |
| StreamMessage (Data/Watermark) | ✅ Done | `src/time.rs` |
| Progress/frontier | ✅ Done | CompletedFrontier, ProgressHandle |

---

### production-cluster-tooling.md
**Status: ✅ Complete**

| Phase | Status | Notes |
|-------|--------|-------|
| 1 – Health (is_ready, is_live) | ✅ Done | `Graph::is_ready()`, `Graph::is_live()` |
| 2 – Structured logging | ✅ Done | tracing in execute, stop, wait_for_completion |
| 3 – Metrics | ✅ Done | Prometheus, streamweave_errors_total |
| 4 – Config/shutdown docs | ✅ Done | §7 in doc |
| 5 – Cluster health | ✅ Done | aggregate_cluster_health, ClusterHealthReport |

---

### progress-tracking.md
**Status: ✅ Complete**

| Component | Status | Notes |
|-----------|--------|-------|
| CompletedFrontier | ✅ Done | Advance, less_than, less_equal |
| ProgressHandle | ✅ Done | execute_with_progress, execute_with_progress_per_sink |
| Watermarks | ✅ Done | Via timestamped path, WatermarkInjectorNode |

---

### scope-in-process-no-distributed-fault-tolerance.md
**Status: ✅ Complete**

| Item | Status | Notes |
|------|--------|-------|
| README scope | ✅ Done | Scope and limitations section |
| Architecture doc | ✅ Done | docs/architecture.md |
| API docs | ✅ Done | "In-process" where relevant |

---

### streamweave-gap-analysis-and-implementation-strategy.md
**Status:** Master document; see table in §1 above and individual design docs for status.

---

### timestamped-differential-dataflow.md
**Status: ✅ Complete**

| Component | Status | Notes |
|-----------|--------|-------|
| DifferentialElement | ✅ Done | `src/time.rs` (or nodes) |
| ToDifferentialNode | ✅ Done | Wrap items as diff +1 |
| DifferentialGroupByNode | ✅ Done | Count aggregation |
| DifferentialJoinNode | ✅ Done | Equi-join over differential streams |

---

### windowing.md
**Status: ✅ Complete**

| Type | Status | Notes |
|------|--------|-------|
| Count-based | ✅ Exists | WindowNode (existing) |
| Processing-time tumbling | ✅ Done | TumblingProcessingTimeWindowNode |
| Event-time tumbling | ✅ Done | TumblingEventTimeWindowNode |
| Event-time sliding | ✅ Done | SlidingEventTimeWindowNode |
| Event-time session | ✅ Done | SessionEventTimeWindowNode |
