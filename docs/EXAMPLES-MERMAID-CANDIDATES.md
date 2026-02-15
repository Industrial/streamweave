# Examples That Can Use Mermaid (`.mmd`) Files

**Status:** The pipeline/feature examples and all node examples have been implemented: each has a matching `.mmd` file and the example loads it. Mermaid is always included (no feature flag). Run e.g. `cargo run --example production_ready`.

This list identifies which examples can be rewritten (or augmented) with a **similarly named `.mmd` file** so the graph topology and metadata are defined in Mermaid and loaded via `parse_mmd_file_to_blueprint` + `blueprint_to_graph` (with a `NodeRegistry` for real nodes or placeholders for structure-only).

Convention: [streamweave-mermaid-convention.md](streamweave-mermaid-convention.md).  
Implementation: `src/mermaid/` (parse, blueprint, blueprint_to_graph, export).

---

## 1. Pipeline / feature examples (high value)

These examples **build a `Graph`** and run it. Each can have a matching `.mmd` that describes the same topology and, where relevant, streamweave comments (I/O, execution_mode, supervision, shard_config, feedback).

| Example | Suggested `.mmd` | Notes |
|--------|------------------|--------|
| **production_ready** | `production_ready.mmd` | Simple: producer → consumer. I/O bindings in comments. |
| **deterministic_execution** | `deterministic_execution.mmd` | Two producers → merge → consumer; `execution_mode=deterministic`. |
| **supervision_restart** | `supervision_restart.mmd` | producer → worker → consumer; `node worker supervision_policy=Restart`. |
| **checkpoint_restore** | `checkpoint_restore.mmd` | producer → stateful → consumer. |
| **event_time_watermark** | `event_time_watermark.mmd` | producer → WatermarkInjectorNode → output. |
| **event_time_window** | `event_time_window.mmd` | producer → WatermarkInjectorNode → TumblingEventTimeWindowNode → output (and optional late port). |
| **differential_stream** | `differential_stream.mmd` | producer → ToDifferentialNode → DifferentialGroupByNode → DifferentialJoinNode → consumer. |
| **bounded_iteration** | `bounded_iteration.mmd` | seed → BoundedIterationNode (inner graph: merge with seed + feedback). Feedback edges and possibly subgraph in comments. |
| **shard_config** | `shard_config.mmd` | Single node (e.g. variable); `shard_config=1/3` in comments. |

Rewriting approach: keep custom nodes (ProducerNode, ConsumerNode, etc.) in Rust; register them in a `NodeRegistry` by kind (e.g. `Producer`, `Consumer`, `MergeNode`, `WatermarkInjectorNode`). Put the same kinds in the `.mmd` via `%% streamweave: node <id> kind=...`. Then the example can load the `.mmd` and build the graph from the blueprint + registry.

---

## 2. Examples that should NOT get a `.mmd`

These do **not** define a runnable graph topology (or only use a single node without a real pipeline), so a `.mmd` file doesn’t add value.

| Example | Reason |
|--------|--------|
| **exactly_once_state** | No `Graph`; demonstrates `KeyedStateBackend`, `StatefulNodeDriver`, snapshot/restore API only. |
| **memoizing_node** | Runs a single `MemoizingMapNode` directly; no graph construction. |

---

## 3. Node examples (optional, lower priority)

There are many **single-node examples** under `examples/nodes/` (stream, string, time, aggregation, etc.). Each builds a tiny graph (typically **graph input → one node → graph output**) with the `graph!` macro.

- **Can add `.mmd`:** For any such example (e.g. `map_node`, `filter_node`), you can add a matching `.mmd` (e.g. `map_node.mmd`) that describes the same topology: one internal node with one input and one output, plus graph I/O in comments.
- **Rewriting:** The example would need to enable the `mermaid` feature, call `parse_mmd_file_to_blueprint`, build a `NodeRegistry` that registers that node’s kind (e.g. `MapNode`), then `blueprint_to_graph` and run. Configuration (e.g. `map_config`) would still be supplied in Rust.

Recommendation: do **pipeline/feature examples** (Section 1) first; then add `.mmd` for a few **representative node examples** (e.g. `map_node`, `filter_node`, `merge_node`) if you want to show the pattern for node-level demos.

---

## 4. Summary

- **Add a similarly named `.mmd` and (optionally) rewrite to load it:**  
  `production_ready`, `deterministic_execution`, `supervision_restart`, `checkpoint_restore`, `event_time_watermark`, `event_time_window`, `differential_stream`, `bounded_iteration`, `shard_config`.
- **Do not add `.mmd`:**  
  `exactly_once_state`, `memoizing_node`.
- **Optional later:**  
  Any of the `examples/nodes/*` examples; start with a few (e.g. `map_node`, `filter_node`) as a template for the rest.
