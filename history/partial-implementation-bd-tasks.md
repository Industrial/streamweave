# Partial implementations – bd tasks (2025-02-15)

Created from `docs/IMPLEMENTATION-STATUS.md`. Each task tracks remaining work for capabilities marked ⚠️ Partial.

## Epics

| ID | Title | Doc | Remaining work |
|----|-------|-----|----------------|
| streamweave-ep7 | Actor supervision: subgraph-as-supervision-unit | actor-supervision-trees.md | Subgraph-as-supervision-unit (hierarchical supervision) |
| streamweave-24v | Cluster sharding: distribution layer runtime | cluster-sharding.md | Built-in deploy N instances, route by partition key |
| streamweave-a39 | Distributed checkpointing: Chandy-Lamport protocol | distributed-checkpointing.md | Barrier-based coordinated checkpoint flow |
| streamweave-4iu | Incremental recomputation: subgraph time-scoped execution | incremental-recomputation.md | execute_for_time_range(plan.nodes, time_range) |

## Backlog task

| ID | Title | Doc | Note |
|----|-------|-----|------|
| streamweave-69d | Auto-scaling: built-in scaler (deferred) | auto-scaling-clusters.md | P4, deferred until needed |

## Dependencies

- streamweave-a39 (Chandy-Lamport) blocks on streamweave-24v (distribution runtime)

## Commands

```bash
devenv shell -- bd ready --json
devenv shell -- bd list --status open
```
