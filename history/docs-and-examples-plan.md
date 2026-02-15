# Plan: Documentation and Examples for Newly Added Features

**Epic:** streamweave-99c — Documentation and examples for all newly added StreamWeave features  
**Created:** 2026-02-15

## Summary

- **1 epic**, **28 child tasks** (11 runnable examples + 16 doc “Quick start / Example” sections + 1 docs index + 1 EXAMPLES-ROADMAP update).
- All tasks are under epic `streamweave-99c`; no cross-task dependencies, so work can proceed in parallel.

## Runnable examples (11 tasks)

| ID | Example | Covers |
|----|---------|--------|
| streamweave-99c.2 | `event_time_watermark` | execute_with_progress, WatermarkInjectorNode, frontier |
| streamweave-99c.3 | `event_time_window` | Tumbling/sliding/session event-time + late-data policy |
| streamweave-99c.4 | `checkpoint_restore` | trigger_checkpoint, restore_from_checkpoint, FileCheckpointStorage |
| streamweave-99c.5 | `deterministic_execution` | ExecutionMode::Deterministic, execute_deterministic |
| streamweave-99c.6 | `supervision_restart` | execute_with_supervision, failure + restart |
| streamweave-99c.7 | `bounded_iteration` | BoundedIterationNode or execute_with_rounds |
| streamweave-99c.8 | `differential_stream` | ToDifferentialNode, DifferentialGroupBy, DifferentialJoin |
| streamweave-99c.9 | `exactly_once_state` | KeyedStateBackend, StatefulNodeDriver, idempotent |
| streamweave-99c.10 | `production_ready` | is_ready, is_live, Prometheus, logging |
| streamweave-99c.11 | `shard_config` | ShardConfig, owns_key, export_state (in-process demo) |
| streamweave-99c.12 | `memoizing_node` | MemoizingMapNode, cache hit/miss |

## Doc updates: “Quick start / Example” section (16 tasks)

Each task adds a minimal code snippet and/or link to the corresponding example.

| ID | Document |
|----|----------|
| streamweave-99c.13 | logical-timestamps-timely-and-streamweave.md |
| streamweave-99c.14 | event-time-semantics.md |
| streamweave-99c.15 | progress-tracking.md |
| streamweave-99c.16 | deterministic-execution.md |
| streamweave-99c.17 | exactly-once-state.md |
| streamweave-99c.18 | windowing.md |
| streamweave-99c.19 | event-time-processing.md |
| streamweave-99c.20 | timestamped-differential-dataflow.md |
| streamweave-99c.21 | cyclic-iterative-dataflows.md |
| streamweave-99c.22 | distributed-checkpointing.md |
| streamweave-99c.23 | cluster-sharding.md |
| streamweave-99c.24 | actor-supervision-trees.md |
| streamweave-99c.25 | incremental-recomputation.md |
| streamweave-99c.26 | production-cluster-tooling.md |
| streamweave-99c.27 | auto-scaling-clusters.md |

## Index and roadmap (2 tasks)

| ID | Task |
|----|------|
| streamweave-99c.1 | Add docs index: Examples and how-to (feature → doc + example link) |
| streamweave-99c.28 | Update docs/EXAMPLES-ROADMAP.md with example checklist and links |

## Suggested execution order

1. **Ready work:** `devenv shell -- bd ready --json` — all 28 subtasks are unblocked.
2. **Examples first (high value):** event_time_watermark → event_time_window → checkpoint_restore → deterministic_execution → …
3. **Doc sections:** Can be done in parallel with examples; add snippet + “Full example: `cargo run --example <name>`.”
4. **Index (streamweave-99c.1):** Can be done early (list features + doc links; add example links as binaries appear).
5. **EXAMPLES-ROADMAP (streamweave-99c.28):** Update as each example is implemented (checklist + links).

## Reference

- **Implementation status:** docs/IMPLEMENTATION-STATUS.md  
- **Examples roadmap:** docs/EXAMPLES-ROADMAP.md  
- **Gap analysis:** docs/streamweave-gap-analysis-and-implementation-strategy.md  
