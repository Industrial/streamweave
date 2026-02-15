# Time-range recomputation – bd tasks (2025-02-15)

Planned from `docs/incremental-recomputation.md` §5.1 and `docs/IMPLEMENTATION-STATUS.md`.

## Epic: streamweave-ulx

**Time-range recomputation (Phase 4)**

Complete incremental-recomputation Phase 4: time-scoped execution using `TimeRange`, `RecomputeRequest`, `nodes_downstream_transitive`, and `CompletedFrontier`.

## Subtasks and dependencies

| ID | Title | Depends on | Notes |
|----|-------|------------|-------|
| streamweave-ulx.1 | Design time-scoped execution API | — | Ready |
| streamweave-ulx.2 | Per-node completed frontier tracking | — | Ready |
| streamweave-ulx.3 | Recompute scheduler | 1, 2 | Uses `nodes_downstream_transitive` |
| streamweave-ulx.4 | Wire time-scoped execution into Graph | 1, 3 | Execution-model changes |
| streamweave-ulx.5 | Tests and docs for time-range recomputation | 4 | Integration tests, doc updates |

## Ready work

- `streamweave-ulx.1` Design time-scoped execution API
- `streamweave-ulx.2` Per-node completed frontier tracking

## Commands

```bash
devenv shell -- bd ready --json
devenv shell -- bd dep tree streamweave-ulx
devenv shell -- bd show streamweave-ulx.1
```
