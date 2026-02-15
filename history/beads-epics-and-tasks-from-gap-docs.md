# Beads epics and tasks from gap-analysis design docs

Created by beads-planner from the 15 detailed design documents (docs/*.md). Each epic corresponds to one design doc; tasks are phased implementation steps with `blocks:` dependencies for correct order.

## Epics (one per design doc)

| Epic ID | Gap § | Title | Doc |
|---------|-------|--------|-----|
| streamweave-sh2 | 2 | Deterministic execution | deterministic-execution.md |
| streamweave-fy8 | 3 | Exactly-once state | exactly-once-state.md |
| streamweave-wa9 | 4 | Scope (in-process, no distributed FT) | scope-in-process-no-distributed-fault-tolerance.md |
| streamweave-bp8 | 5 | Cyclic iterative dataflows | cyclic-iterative-dataflows.md |
| streamweave-9ag | 6 | Actor supervision trees | actor-supervision-trees.md |
| streamweave-ohv | 7 | Production cluster tooling | production-cluster-tooling.md |
| streamweave-9qb | 8 | Event-time semantics | event-time-semantics.md |
| streamweave-sua | 9 | Progress tracking (watermarks) | progress-tracking.md |
| streamweave-cqr | 10 | Timestamped differential dataflow | timestamped-differential-dataflow.md |
| streamweave-um4 | 11 | Windowing | windowing.md |
| streamweave-3we | 12 | Event-time processing | event-time-processing.md |
| streamweave-4fs | 13 | Incremental recomputation | incremental-recomputation.md |
| streamweave-gjz | 14 | Cluster sharding | cluster-sharding.md |
| streamweave-g43 | 15 | Distributed checkpointing | distributed-checkpointing.md |
| streamweave-g4g | 16 | Auto-scaling clusters | auto-scaling-clusters.md |

(One duplicate epic streamweave-4rq was closed as duplicate of streamweave-9ag.)

## Tasks by epic (implementation order within epic)

### §2 Deterministic (streamweave-sh2)
- **sh2.1** Document determinism contract in README/ARCHITECTURE
- **sh2.2** Implement ExecutionMode::Deterministic and execute_deterministic() — *blocks: sh2.1* (note: may show blocks:sh2 epic; fix if needed)
- **sh2.3** Add determinism tests (same input same output) — *blocks: sh2.2*
- **sh2.4** Add deterministic merge for multi-input nodes — *blocks: sh2.3*

### §3 Exactly-once (streamweave-fy8)
- **fy8.1** Define exactly-once state contract and key/version semantics
- **fy8.2** Implement in-process state backend put/get/snapshot/restore — *blocks: fy8.1*
- **fy8.3** Add stateful node helper using LogicalTime as version — *blocks: fy8.2*
- **fy8.4** Document idempotent sink pattern — *blocks: fy8.3*

### §4 Scope (streamweave-wa9)
- **wa9.1** Add Scope section to README (in-process, no distributed FT)
- **wa9.2** Add execution and failure model to architecture doc — *blocks: wa9.1*

### §5 Cyclic (streamweave-bp8)
- **bp8.1** Design and document bounded iteration API
- **bp8.2** Implement BoundedIterationNode with buffered feedback — *blocks: bp8.1*
- **bp8.3** Add cyclic tests (fixed-point, rounds) — *blocks: bp8.2*

### §6 Supervision (streamweave-9ag)
- **9ag.1** Define SupervisionPolicy and FailureAction
- **9ag.2** Catch node task failure and send to supervisor — *blocks: 9ag.1*
- **9ag.3** Implement supervisor loop restart node on failure — *blocks: 9ag.2*
- **9ag.4** Add restart group and escalate — *blocks: 9ag.3*

### §7 Production tooling (streamweave-ohv)
- **ohv.1** Add readiness and liveness semantics or is_ready()
- **ohv.2** Integrate structured logging (tracing) for key events — *blocks: ohv.1*
- **ohv.3** Add metrics counters and gauges Prometheus or OTel — *blocks: ohv.2*
- **ohv.4** Document config format and graceful shutdown — *blocks: ohv.3*

### §8 Event-time (streamweave-9qb)
- **9qb.1** Document event time vs processing time and convention
- **9qb.2** Allow timestamped path to use external event time — *blocks: 9qb.1*
- **9qb.3** Add EventTimeExtractorNode or event-time input — *blocks: 9qb.2*

### §9 Progress (streamweave-sua)
- **sua.1** Document current progress and add per-sink frontier option
- **sua.2** Introduce StreamMessage Data or Watermark type — *blocks: sua.1*
- **sua.3** Add watermark injection at sources and consumption in windows — *blocks: sua.2*

### §10 Differential (streamweave-cqr)
- **cqr.1** Define DifferentialElement and differential execution path
- **cqr.2** Wrap existing nodes to produce payload time diff +1 — *blocks: cqr.1*
- **cqr.3** Implement differential group-by node — *blocks: cqr.2*
- **cqr.4** Implement differential join node — *blocks: cqr.3*

### §11 Windowing (streamweave-um4)
- **um4.4** Add processing-time tumbling window node
- **um4.5** Implement tumbling event-time window close on watermark — *blocks: um4.4, sua.2*
- **um4.7** Add sliding and session event-time windows — *blocks: um4.5*
- **um4.8** Add late-data policy drop or side output — *blocks: um4.7* (created with blocks:um4.6; logical predecessor is um4.7)

### §12 Event-time processing (streamweave-3we)
- **3we.1** Wire event-time flow and document late-data policy — *deps: 9qb.3, sua.3, um4.5* (cross-epic; may be unset)
- **3we.2** Add event-time processing tests out-of-order and late — *blocks: 3we.1*

### §13 Incremental (streamweave-4fs)
- **4fs.1** Document incremental patterns replay memoization
- **4fs.2** Add optional memoizing wrapper node for deterministic keyed nodes — *blocks: 4fs.1*

### §14 Sharding (streamweave-gjz)
- **gjz.1** Define partitioning contract in API
- **gjz.2** Add shard_id and key range to execution config — *blocks: gjz.1*
- **gjz.3** Document or implement distribution layer N processes — *blocks: gjz.2*
- **gjz.4** State migration export import for rebalance — *blocks: gjz.3, fy8.2*

### §15 Checkpointing (streamweave-g43)
- **g43.1** Local checkpoint state backend snapshot and positions
- **g43.2** Restore from checkpoint and resume from positions — *blocks: g43.1*
- **g43.3** Periodic checkpoint trigger — *blocks: g43.2*

### §16 Auto-scaling (streamweave-g4g)
- **g4g.1** Document auto-scaling as future goal and deps (P4, defer)

## Cross-epic dependency order (from gap analysis)

- **Event-time processing (§12)** needs: Event-time (§8), Progress (§9), Windowing event-time (§11). So 3we.1 logically after 9qb.3, sua.3, um4.5.
- **Windowing event-time** needs: Progress (sua.2 for watermarks). um4.5 blocks on sua.2.
- **Sharding state migration** needs: Exactly-once state (fy8.2). gjz.4 blocks on fy8.2.
- **Distributed checkpointing** (later phases) would block on local checkpointing and sharding; only local phases created.

## How to work on this

1. **Ready work:** `devenv shell -- bd ready --json` — lists issues with no open blockers.
2. **Claim:** `devenv shell -- bd update <id> --status in_progress --claim --json`
3. **Implement** using the referenced doc in `docs/`.
4. **Close:** `devenv shell -- bd close <id> --reason "Done" --json`
5. **Commit** `.beads/issues.jsonl` with code changes.

## Suggested execution order (first ready tasks)

Epics with no task dependencies (or whose first task has no blocks) can start immediately:

- **wa9.1**, **wa9.2** (Scope doc)
- **sh2.1** (Determinism contract)
- **fy8.1** (Exactly-once contract)
- **bp8.1** (Cyclic design)
- **9ag.1** (Supervision policy)
- **ohv.1** (Health/ready)
- **9qb.1** (Event-time doc)
- **sua.1** (Progress doc)
- **cqr.1** (Differential type)
- **um4.4** (Processing-time window)
- **4fs.1** (Incremental doc)
- **gjz.1** (Partitioning contract)
- **g43.1** (Local checkpoint)
- **g4g.1** (Auto-scaling doc)

Then proceed in dependency order within each epic and across epics as above.
