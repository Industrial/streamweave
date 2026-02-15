# Examples roadmap

Examples for each newly supported feature. Run any with: `cargo run --example <name>`.

---

## Checklist: feature examples

| Done | Example | Feature | Description | Source |
|------|---------|---------|-------------|--------|
| ✅ | `event_time_watermark` | Progress tracking | `execute_with_progress`, `WatermarkInjectorNode`, frontier | [event_time_watermark.rs](../examples/event_time_watermark.rs) |
| ✅ | `event_time_window` | Event-time windowing | Tumbling event-time windows, watermarks, late-data policy | [event_time_window.rs](../examples/event_time_window.rs) |
| ✅ | `checkpoint_restore` | Local checkpointing | `trigger_checkpoint`, `restore_from_checkpoint`, `FileCheckpointStorage` | [checkpoint_restore.rs](../examples/checkpoint_restore.rs) |
| ✅ | `deterministic_execution` | Deterministic execution | `ExecutionMode::Deterministic`, `execute_deterministic` | [deterministic_execution.rs](../examples/deterministic_execution.rs) |
| ✅ | `supervision_restart` | Actor supervision | `execute_with_supervision`, failure + restart | [supervision_restart.rs](../examples/supervision_restart.rs) |
| ✅ | `bounded_iteration` | Cyclic dataflows | `BoundedIterationNode`, `execute_with_rounds` | [bounded_iteration.rs](../examples/bounded_iteration.rs) |
| ✅ | `differential_stream` | Timestamped differential | `ToDifferentialNode`, `DifferentialGroupByNode`, `DifferentialJoinNode` | [differential_stream.rs](../examples/differential_stream.rs) |
| ✅ | `exactly_once_state` | Exactly-once state | `KeyedStateBackend`, `StatefulNodeDriver`, snapshot/restore, idempotent sink | [exactly_once_state.rs](../examples/exactly_once_state.rs) |
| ✅ | `production_ready` | Production tooling | `is_ready`, `is_live`, Prometheus metrics | [production_ready.rs](../examples/production_ready.rs) |
| ⬜ | `shard_config` | Cluster sharding (API) | `ShardConfig`, `owns_key`, `export_state` (in-process demo) | [shard_config.rs](../examples/shard_config.rs) |
| ⬜ | `memoizing_node` | Incremental recomputation | `MemoizingMapNode`, cache hit/miss | [memoizing_node.rs](../examples/memoizing_node.rs) |

---

## Implementation order (reference)

1. event_time_watermark → 2. event_time_window → 3. checkpoint_restore → 4. deterministic_execution → 5. supervision_restart → 6. bounded_iteration → 7. differential_stream → 8. exactly_once_state → 9. production_ready → 10. shard_config → 11. memoizing_node.

---

## Other examples

| Example | Covers |
|---------|--------|
| `graph_macro_simple` | Graph macro, linear pipeline |
| `graph_macro_fan_patterns` | Fan-in, multiple sources |
| `graph_macro_io` | Graph I/O, external channels |

See also [EXAMPLES-AND-HOW-TO.md](EXAMPLES-AND-HOW-TO.md) (if present) and the “Quick start / Example” sections in each feature doc (e.g. [event-time-semantics.md](event-time-semantics.md), [progress-tracking.md](progress-tracking.md), [windowing.md](windowing.md)).
