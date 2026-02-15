# StreamWeave–Mermaid Convention

This document defines how StreamWeave graphs are encoded in Mermaid flowchart (`.mmd`) source. Parser and exporter must both follow this convention for roundtrip. See [mermaid-streamweave-implementation-plan.md](mermaid-streamweave-implementation-plan.md) for the overall plan.

---

## 1. Edge-label format and port escaping

Every StreamWeave edge is represented as one Mermaid edge. **Port names are required** and are encoded in the edge label.

### 1.1 Format

- **Syntax:** `source_port->target_port`
- **Example:** `out->in` means the edge goes from the source node’s output port named `out` to the target node’s input port named `in`.
- The arrow `->` is the **separator** between source and target port names. It must appear exactly once per label.

### 1.2 Escaping

Port names are arbitrary strings in StreamWeave and may contain characters that conflict with the label format or Mermaid syntax.

- **Reserved in label:** The character sequence `->` is the separator. If a port name contains a literal `->`, it must be escaped.
- **Escape scheme:** Use backslash escaping:
  - `\` → `\\`
  - `->` → `\->` (so a port name containing `->` is written as `\->` in its segment)
- **Parsing:** When reading an edge label, split on the first unescaped `->`. Then unescape each segment (replace `\\` with `\`, `\->` with `->`) to obtain `source_port` and `target_port`.
- **Special characters in node IDs:** Mermaid node IDs are typically alphanumeric and underscore. StreamWeave node names should be restricted to the same set for `.mmd` roundtrip, or we document that node names containing spaces/special chars are quoted/escaped per Mermaid’s rules (e.g. in parentheses). For Phase 1, assume node IDs are safe (alphanumeric and `_`).

### 1.3 Examples

| Source port | Target port | Edge label   |
|-------------|-------------|--------------|
| `out`       | `in`        | `out->in`    |
| `result`    | `value`     | `result->value` |
| `out`       | `in` (port name is literally `->`) | `out->\->` (target port = `->`) |

(If the source port were the literal `->`, the label would be `\->->in`.)

---

## 2. Comment grammar (`%% streamweave:`)

All StreamWeave-specific metadata and graph I/O bindings are encoded in Mermaid **comment lines** so that a single `.mmd` file is self-contained. Every such line starts with the prefix `%% streamweave:` (after optional whitespace). The rest of the line is parsed according to the grammar below.

### 2.1 Graph I/O

- **Input binding:** `input <external_name> -> <node_id>.<port_name>`
  - Example: `input config -> source.value`
  - Optional: ` [<initial_value>]` at end (for future use; parser may ignore).
- **Output binding:** `output <external_name> <- <node_id>.<port_name>`
  - Example: `output result <- sink.out`
- These lines define which internal node ports are exposed as the graph’s input/output ports. Parser and exporter read/write them in a **comment block** at the top of the flowchart (e.g. before the first `flowchart` or node).

### 2.2 Execution and sharding

- **Execution mode:** `execution_mode=concurrent` | `execution_mode=deterministic`
  - Default if omitted: `concurrent`.
- **Shard config:** `shard_config=<shard_id>/<total_shards>`
  - Example: `shard_config=0/4`
  - Only relevant when running multiple graph instances (e.g. cluster sharding).

### 2.3 Supervision

- **Per-node supervision:** `node <node_id> supervision_policy=<Restart|Stop|Escalate> [supervision_group=<group_id>]`
  - Example: `node worker supervision_policy=Restart supervision_group=grp1`
- **Subgraph as supervision unit:** `subgraph_unit <subgraph_id>`
  - The given subgraph (nested graph) is treated as one unit for restart/stop.

### 2.4 Feedback edges (cycles)

- **Feedback edge:** `feedback <source_id> <source_port> <target_id> <target_port>` or `feedback <edge_id>` if we assign explicit edge ids.
  - For simplicity, Phase 1 can use: `feedback <source_id>-><target_id>` and match the edge by (source, target) since port is already on the edge label. Alternatively: document that edge labels prefixed with `fb_` (e.g. `fb_out->in`) denote feedback edges.
  - Parser passes feedback info to the graph’s cycle/round API; exporter writes it back so roundtrip preserves it.

### 2.5 Node kind (import only)

- **Node kind for registry:** `node <node_id> kind=<KindName>`
  - Example: `node mapper kind=MapNode`
  - Used when building a runnable graph from a registry; optional. If absent, the node is a placeholder or the caller supplies instances.

### 2.6 Node ID mapping (export with mermaid-builder)

- When the exporter uses the `mermaid-builder` crate, the flowchart uses internal numeric node ids (`v0`, `v1`, …). So that the parser can restore StreamWeave node ids, the comment block may include:
  - **Mapping:** `node_id <internal_id>=<streamweave_node_id>`
  - Example: `node_id v0=a`, `node_id v1=b`
  - Nodes are listed in deterministic (e.g. sorted) order. The parser should use these lines to map `v0` → `a`, `v1` → `b`, etc., when building the blueprint.

---

## 3. Subgraph mapping and external ports

- **Subgraphs:** A Mermaid `subgraph <id> ... end` block represents a **nested StreamWeave graph** (Graph-as-Node). The subgraph’s `<id>` is the node name of that graph when used as a node in the parent.
- **External ports:** Edges that **cross the subgraph boundary** define the nested graph’s inputs and outputs:
  - **Edge from outside into the subgraph:** The target (node, port) inside the subgraph is an **input** of the nested graph. The external name for that input can be given in the `%% streamweave:` block (e.g. `input <external_name> -> <node>.<port>` where `<node>` is inside the subgraph).
  - **Edge from inside the subgraph to outside:** The source (node, port) inside the subgraph is an **output** of the nested graph. Similarly, `output <external_name> <- <node>.<port>` in the comment block names it.
- **Naming rule:** When the same `.mmd` file describes both the parent and the subgraph, graph I/O comment lines that reference nodes inside a subgraph apply to that subgraph’s external ports. We document that subgraph-scoped I/O lines may be prefixed with the subgraph id (e.g. `%% streamweave: subgraph <id>`) or listed in a block that follows the subgraph definition, so the parser can attach bindings to the correct nested blueprint.
- **Roundtrip:** Exporter emits nested graphs as `subgraph <id> ... end` and emits the corresponding graph I/O comment lines for edges crossing the boundary so that re-parsing reproduces the same structure.

---

## 4. Registry vs placeholders

When building a runnable graph from a blueprint (e.g. after parsing a `.mmd` file), the implementation supports two modes:

- **Without a node registry:** The function `blueprint_to_graph(blueprint)` uses **placeholder nodes** for every blueprint node. Placeholders implement the StreamWeave `Node` interface with the correct input/output port names but produce **no data** when executed (empty streams). The resulting graph is suitable for:
  - **Structure validation** (topology, I/O bindings, execution mode, shard config),
  - **Roundtrip** (parse → export → parse) to verify that `.mmd` and blueprint stay in sync.
  The graph is **not** meant for real execution; it is for tooling, tests, and roundtrip only.

- **With a node registry (optional, future):** A registry can map node kind (e.g. from `%% streamweave: node <id> kind=MapNode`) to a constructor that returns `Box<dyn Node>`. Then `blueprint_to_graph` can instantiate real nodes and the resulting graph can be executed. Until a registry is implemented, only placeholders are used.

---

## 5. Optional sidecar file (`*.streamweave.yaml`)

When the comment block in the `.mmd` would be too large or you want to reference external config, you can use a **sidecar file** next to the diagram: same base path with extension `.streamweave.yaml` (e.g. `pipeline.mmd` → `pipeline.streamweave.yaml`).

### 5.1 When to use

- **Comment block preferred:** For small pipelines, keep everything in the `.mmd` file (comment block only) so the diagram is self-contained.
- **Sidecar useful when:** You have many graph I/O bindings, per-node metadata, or node kinds; or you want to generate/merge the sidecar from external tooling.

### 5.2 Schema

The sidecar is a YAML object. All fields are optional. If both the comment block and a sidecar are present, the **sidecar overrides** the comment block for the fields it defines (merge semantics: sidecar I/O and metadata replace or extend as documented below).

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Graph name. |
| `inputs` | array of `{ external_name, node_id, port_name }` | Graph input bindings. |
| `outputs` | array of `{ external_name, node_id, port_name }` | Graph output bindings. |
| `execution_mode` | `"concurrent"` \| `"deterministic"` | Default: `concurrent`. |
| `shard_config` | `{ shard_id, total_shards }` | When running multiple instances. |
| `nodes` | object (node_id → `{ kind?, label?, supervision_policy?, supervision_group? }`) | Per-node kind/label and supervision. |
| `subgraph_units` | array of string | Subgraph ids that are supervision units. |
| `feedback_edge_ids` | array of string | Edge identifiers for feedback (cycles). |

Example:

```yaml
name: my_pipeline
execution_mode: deterministic
inputs:
  - external_name: config
    node_id: source
    port_name: value
outputs:
  - external_name: result
    node_id: sink
    port_name: out
nodes:
  mapper:
    kind: MapNode
  worker:
    supervision_policy: Restart
    supervision_group: grp1
```

### 5.3 Parser and exporter

- **Parser:** When loading a `.mmd` file, the caller may pass the diagram path; if a co-located `*.streamweave.yaml` exists, it is read and merged into the blueprint (sidecar overrides comment block for overlapping fields).
- **Exporter:** When exporting a blueprint to `.mmd`, the caller may pass an output path; the exporter can optionally write a co-located `*.streamweave.yaml` with the same I/O and metadata so that roundtrip (parse → export → parse) preserves structure when using the sidecar.
