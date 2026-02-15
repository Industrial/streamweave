# Examples roadmap

Examples to add for each newly supported feature. Current examples: `graph_macro_simple`, `graph_macro_fan_patterns`, `graph_macro_io`.

---

## Recommended new examples

| Example | Feature | Description | Priority |
|---------|---------|-------------|----------|
| `event_time_window` | Event-time processing, windowing | Tumbling/sliding/session event-time windows with watermarks, late-data policy | High |
| `event_time_watermark` | Progress tracking, watermarks | `execute_with_progress`, `WatermarkInjectorNode`, frontier observation | High |
| `deterministic_execution` | Deterministic execution | `ExecutionMode::Deterministic`, `execute_deterministic`, reproducible runs | Medium |
| `checkpoint_restore` | Local checkpointing | `trigger_checkpoint`, `restore_from_checkpoint`, `Node::restore_state` | High |
| `supervision_restart` | Actor supervision | `execute_with_supervision`, `set_node_supervision_policy`, failure + restart | Medium |
| `bounded_iteration` | Cyclic dataflows | `BoundedIterationNode`, fixed-point iteration (e.g. x := x/2 + 1) | Medium |
| `differential_stream` | Timestamped differential | `ToDifferentialNode`, `DifferentialGroupByNode`, `DifferentialJoinNode` | Medium |
| `exactly_once_state` | Exactly-once state | `KeyedStateBackend`, `StatefulNodeDriver`, idempotent updates | Medium |
| `shard_config` | Cluster sharding (API) | `ShardConfig`, `owns_key`, `export_state_for_keys` (in-process demo) | Low |
| `memoizing_node` | Incremental recomputation | `MemoizingMapNode`, cache hit/miss behavior | Low |
| `production_ready` | Production tooling | `is_ready`, `is_live`, Prometheus metrics, structured logging | Medium |

---

## Implementation order

1. **event_time_window** – Core streaming feature; demonstrates event-time semantics end-to-end.
2. **event_time_watermark** – Prerequisite for event-time; shows progress/watermark usage.
3. **checkpoint_restore** – Fault-tolerance story; clear value for users.
4. **deterministic_execution** – Testing/reproducibility; small example.
5. **supervision_restart** – Failure handling; inject failure and show restart.
6. **bounded_iteration** – Cyclic dataflow; distinctive vs. DAG-only systems.
7. **differential_stream** – Incremental operators; differential group-by and join.
8. **exactly_once_state** – State correctness; keyed state, versioning.
9. **production_ready** – Observability; health, metrics, logging.
10. **shard_config** – Sharding API; `SHARD_ID` env, `owns_key` filtering.
11. **memoizing_node** – Incremental; MemoizingMapNode cache behavior.

---

## Existing examples coverage

| Example | Covers |
|---------|--------|
| `graph_macro_simple` | Graph macro, linear pipeline |
| `graph_macro_fan_patterns` | Fan-in, multiple sources |
| `graph_macro_io` | Graph I/O, external channels |

**Gaps:** No examples for event-time, progress, determinism, checkpointing, supervision, cyclic iteration, differential dataflow, exactly-once state, sharding API, or production tooling.
