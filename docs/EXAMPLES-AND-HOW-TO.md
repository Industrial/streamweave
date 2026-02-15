# Examples and how-to index

Single index of StreamWeave features with links to design docs and runnable examples. For implementation status and roadmap, see [IMPLEMENTATION-STATUS.md](IMPLEMENTATION-STATUS.md) and [EXAMPLES-ROADMAP.md](EXAMPLES-ROADMAP.md).

---

## Feature → design doc and example

| Feature | Design doc | Example |
|--------|------------|--------|
| **Logical timestamps** | [logical-timestamps-timely-and-streamweave.md](logical-timestamps-timely-and-streamweave.md) | — (core model; see graph examples) |
| **Event-time semantics** | [event-time-semantics.md](event-time-semantics.md) | Planned: `event_time_watermark`, `event_time_window` |
| **Progress tracking (watermarks)** | [progress-tracking.md](progress-tracking.md) | Planned: `event_time_watermark` |
| **Event-time processing & windowing** | [event-time-processing.md](event-time-processing.md), [windowing.md](windowing.md) | Planned: `event_time_window` |
| **Deterministic execution** | [deterministic-execution.md](deterministic-execution.md) | Planned: `deterministic_execution` |
| **Exactly-once state** | [exactly-once-state.md](exactly-once-state.md) | Planned: `exactly_once_state` |
| **Scope (in-process)** | [scope-in-process-no-distributed-fault-tolerance.md](scope-in-process-no-distributed-fault-tolerance.md) | — (architectural) |
| **Local checkpointing** | [distributed-checkpointing.md](distributed-checkpointing.md) | Planned: `checkpoint_restore` |
| **Actor supervision (restart)** | [actor-supervision-trees.md](actor-supervision-trees.md) | Planned: `supervision_restart` |
| **Cyclic / iterative dataflows** | [cyclic-iterative-dataflows.md](cyclic-iterative-dataflows.md) | Planned: `bounded_iteration` |
| **Timestamped differential dataflow** | [timestamped-differential-dataflow.md](timestamped-differential-dataflow.md) | Planned: `differential_stream` |
| **Cluster sharding** | [cluster-sharding.md](cluster-sharding.md) | Planned: `shard_config` |
| **Production tooling** | [production-cluster-tooling.md](production-cluster-tooling.md) | Planned: `production_ready` |
| **Incremental recomputation** | [incremental-recomputation.md](incremental-recomputation.md) | Planned: `memoizing_node` |
| **Graph macro & I/O** | [architecture.md](architecture.md), README | See below |

---

## Runnable examples (available today)

Run with: `cargo run --example <name>` (from repo root, inside `devenv shell` if using devenv).

| Example | Description |
|---------|-------------|
| `graph_macro_simple` | Linear pipeline with the `graph!` macro |
| `graph_macro_fan_patterns` | Fan-in patterns with `graph!` |
| `graph_macro_io` | Graph I/O and external channels |
| (node examples) | Many single-node examples under `examples/nodes/` (stream, string, time, etc.); see `cargo run --example <name>` for `map_node`, `filter_node`, `range_node`, etc. |

---

## Quick links

- [Implementation status](IMPLEMENTATION-STATUS.md) – What is implemented vs. planned per capability
- [Examples roadmap](EXAMPLES-ROADMAP.md) – Recommended new examples and implementation order
- [API docs (docs.rs)](https://docs.rs/streamweave) – Full API reference
