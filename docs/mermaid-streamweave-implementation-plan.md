# Mermaid ↔ StreamWeave Implementation Plan

**Goal:** Use `.mmd` (Mermaid flowchart) files as a declarative format for StreamWeave graphs: parse `.mmd` into runnable graphs and export existing graphs to `.mmd`. No rendering to SVG/PNG—only source-code roundtrip.

**Two-AST approach:** We use two crates with different internal representations:

- **mermaid-rs-renderer:** Parse `.mmd` → its own IR (nodes, edges, clusters). We do **not** render; we stop at the parsed IR and map it to StreamWeave.
- **mermaid-builder:** Build Mermaid flowchart syntax from Rust (nodes, edges, subgraphs). We use it only for **export:** StreamWeave graph → `.mmd` string.

The **canonical in-memory representation** is StreamWeave’s `Graph` / `GraphBuilder`. The parse path produces a structure we can turn into a `Graph` (with a node registry or placeholders). The export path consumes a `Graph` (or a serializable view of it) and produces `.mmd` via mermaid-builder. We do **not** need a single shared Mermaid AST; we bridge both crates through StreamWeave’s types.

---

## 1. Scope

### In scope

- **Import:** Parse a flowchart `.mmd` file and produce a StreamWeave graph (topology + metadata). Node instances may be placeholders or come from a registry keyed by node kind/name.
- **Export:** Given a StreamWeave graph (or equivalent descriptor), emit `.mmd` source that conforms to our convention.
- **Convention:** Document how we encode ports, graph I/O, subgraphs, and metadata (execution mode, sharding, supervision, feedback edges) in Mermaid so that roundtrip is well-defined.

### Out of scope

- Rendering `.mmd` to SVG/PNG (use mermaid-rs-renderer or mmdr CLI separately if needed).
- Parsing/generating other Mermaid diagram types (sequence, class, etc.); only flowchart.
- Editing `.mmd` in an IDE with live sync (future possibility only).

---

## 2. StreamWeave–Mermaid Convention

A single, explicit convention keeps import and export consistent and enables roundtrip.

### 2.1 Structure

- **Nodes:** One Mermaid node per StreamWeave node. Node ID (and optional label) = StreamWeave node name.
- **Edges:** One Mermaid edge per StreamWeave edge. **Port names** are required; we encode them in the **edge label** in a fixed format, e.g. `source_port->target_port` (e.g. `out->in`). Parser and exporter must use the same format and escaping rules for port names containing `->` or special characters.
- **Subgraphs:** Mermaid `subgraph <id> ... end` = nested StreamWeave graph (Graph-as-Node). The subgraph’s external ports are those edges that cross the subgraph boundary; we need a way to name them (see Graph I/O below).

### 2.2 Graph I/O

- **Graph inputs:** Represent by either (a) a special node type/label convention (e.g. node id `graph.<input_name>` or a reserved prefix) with an edge to the internal node (edge label = `->target_port`), or (b) a comment block listing `input <external_name> -> <node>.<port> [optional initial value]`. Option (b) is easier to parse and roundtrip without overloading Mermaid syntax.
- **Graph outputs:** Similarly, either a special node `graph.<output_name>` with an edge from the internal node (edge label = `source_port->`), or a comment block `output <external_name> <- <node>.<port>`.
- **Recommendation:** Use a **comment block** at the top of the flowchart (e.g. `%% streamweave: input/output`) with one line per binding so we don’t rely on fake nodes and edge directions. Parser and exporter both read/write this block.

### 2.3 Metadata (execution, sharding, supervision, feedback)

- **Execution mode:** e.g. `%% streamweave: execution_mode=deterministic` (default: concurrent).
- **Shard config:** e.g. `%% streamweave: shard_config=0/4` (shard_id/total_shards); only relevant when running multiple instances.
- **Per-node supervision:** e.g. `%% streamweave: node <id> supervision_policy=Restart supervision_group=grp1` or a sidecar (see below).
- **Subgraph as supervision unit:** e.g. `%% streamweave: subgraph_unit <subgraph_id>`.
- **Feedback edges (for cycles):** Mark edges that are “feedback” for round-based execution, e.g. `%% streamweave: feedback <edge_id>` or edge id prefix `fb_` and document that edges with that id are feedback. Parser passes this to the graph’s cycle/round API; exporter writes it back.

All of the above can live in `%%` comments with a single prefix (e.g. `%% streamweave:`) and a small grammar, or in an optional **sidecar file** (e.g. `pipeline.streamweave.yaml`) for richer metadata. Phase 1 should define the minimal comment-based grammar so that a single `.mmd` file is self-contained for simple cases.

### 2.4 Node kinds (import only)

- The `.mmd` file describes **topology** (nodes and edges with port names). It does not define Rust types. On import we need a way to obtain `Box<dyn Node>` for each node name or “kind”.
- **Options:** (a) **Node registry:** map node id (or a “kind” from label/comment) to a constructor so we build real nodes. (b) **Placeholder nodes:** parse into a “skeleton” graph (e.g. a struct with nodes as string ids and edges) and let the caller attach real nodes later. (c) **Config + registry:** `.mmd` (or sidecar) lists `node <id> kind=MapNode` and we look up `MapNode` in a registry. Recommendation: support at least (b) for a first version (parse to a descriptor/GraphBuilder-like structure that can be turned into a Graph once the caller supplies nodes), and optionally (c) with a registry for tests and tooling.

---

## 3. Implementation Phases

### Phase 1: Convention and internal types (no new deps)

- **Deliverable:** A `docs/` (or `src/mermaid/`) spec document that defines:
  - Exact edge-label format for ports (and escaping).
  - Comment grammar for graph I/O and metadata (execution_mode, shard_config, supervision, feedback, subgraph_unit).
  - How subgraphs map to nested graphs and how external ports are named (which edges are graph input/output for that subgraph).
- **Types:** Introduce an intermediate “Mermaid descriptor” or “Graph blueprint” type (e.g. in `src/mermaid/`) that holds: graph name, list of node ids (and optional kind/label), list of edges with (source, source_port, target, target_port), graph input/output bindings, and metadata (execution mode, shard config, per-node supervision, feedback edge ids, subgraph units). This type is the target of the **parser** and the source for the **exporter** when we don’t start from a live `Graph` (e.g. when exporting from a blueprint we just parsed).
- **No mermaid-rs-renderer or mermaid-builder yet.** This phase is convention + types only, and unit tests that build the descriptor by hand and assert the convention strings we expect (e.g. a minimal `to_mermaid_string()` on the descriptor that follows the spec, implemented by hand without mermaid-builder).

### Phase 2: Export path (StreamWeave → .mmd)

- **Dependency:** Add `mermaid-builder` (and optionally `mermaid-rs-renderer` with default features off if we want it in the same crate for future use).
- **Implement:** A module that takes either (a) a StreamWeave `Graph` (read-only topology: node names, edges, input/output bindings, and metadata we can expose) or (b) the internal “graph blueprint” from Phase 1, and produces a Mermaid flowchart string.
  - Use mermaid-builder’s flowchart API to add nodes and edges; set **edge labels** to our `source_port->target_port` format.
  - Emit subgraphs for nested graphs (Graph-as-Node); document how we name and emit external ports (comment block or convention).
  - Emit the `%% streamweave: ...` block (and any per-node/per-edge metadata) at the top or in comments so the file is self-contained.
- **Output:** `.mmd` source string (caller can write to a file). No rendering.
- **Tests:** Export a few known graphs (linear, fan-in, one with graph I/O, one with a nested graph) and snapshot the `.mmd` or assert it contains expected lines and our comment block.

### Phase 3: Parse path (.mmd → blueprint / GraphBuilder)

- **Dependency:** Use `mermaid-rs-renderer` with default features off (no CLI/PNG) so we only get parse + IR.
- **Implement:** A module that:
  - Calls `parse_mermaid(&mmd_string)` (or the crate’s public parse API).
  - Maps the crate’s IR (nodes, edges, clusters/subgraphs) into our **graph blueprint** from Phase 1:
    - Each vertex → node id (and optional label for display).
    - Each edge → (source_id, target_id); **parse edge label** into `source_port`, `target_port` using our convention (fail or fallback if missing/invalid).
    - Subgraphs → nested blueprints; edges crossing subgraph boundary → graph input or graph output for that subgraph (need a rule: e.g. edge from outside into subgraph = input, edge from subgraph to outside = output; port names from edge labels).
  - Parses the `%% streamweave: ...` comment block (and optional sidecar) to fill in graph I/O bindings and metadata (execution_mode, shard_config, supervision, feedback).
- **Output:** Our graph blueprint (or a `GraphBuilder`-like builder that we can feed into `Graph::new` + `add_node`/`add_edge`/`expose_*` once we have node instances). No `Box<dyn Node>` yet—that’s Phase 4.
- **Tests:** Parse `.mmd` fixtures (including those emitted in Phase 2) and assert blueprint structure and metadata; optionally assert that re-export matches the original `.mmd` up to comment/whitespace normalization.

### Phase 4: From blueprint to runnable Graph (node registry / placeholders)

- **Implement:** A way to turn a **graph blueprint** into a runnable `Graph`:
  - **Option A – Placeholder nodes:** For each node id in the blueprint, add a minimal “placeholder” or “stub” node (e.g. a node that implements `Node` but no-ops or errors if executed). This allows “load .mmd and get a Graph” for structure validation and export roundtrip without a registry.
  - **Option B – Registry:** Allow the caller to pass a registry that maps node id (or kind parsed from comments) to a constructor. Parser attaches a “kind” to each node if we add `%% streamweave: node <id> kind=...`; then we look up the kind and instantiate. Useful for tests and for tooling that knows about a fixed set of node types.
- **Deliverable:** Function(s) `blueprint_to_graph(blueprint, node_registry?)` that return a `Graph` (or `Result<Graph, ...>`) so that “parse .mmd → blueprint → graph” is an end-to-end path. Document that without a registry, only placeholder nodes are used and the graph is not meant for real execution.

### Phase 5: Roundtrip and sidecar (optional)

- **Roundtrip tests:** Parse a `.mmd` → blueprint → export to `.mmd` → parse again; assert structural equality (and that metadata is preserved). Test with and without optional sidecar.
- **Sidecar:** If we introduce `*.streamweave.yaml`, define its schema (graph I/O, metadata, node kinds) and document when to use it (e.g. when comment block would be too large or when we want to reference external config). Parser and exporter both support reading/writing the sidecar when the file exists next to the `.mmd`.

---

## 4. Module layout (suggested)

- **`src/mermaid/`** (or `streamweave_mermaid` if this is a separate crate later):
  - `convention.rs` – Constants and format helpers (edge label format, comment prefix, parsing helpers for comment block).
  - `blueprint.rs` – Blueprint/descriptor types (graph name, nodes, edges, graph I/O, metadata).
  - `export.rs` – StreamWeave graph (or blueprint) → Mermaid string using mermaid-builder.
  - `parse.rs` – Mermaid string → blueprint using mermaid-rs-renderer and convention.
  - `roundtrip.rs` – (Phase 5) Helpers or tests for roundtrip.
- **Dependencies:** `mermaid-builder` for export; `mermaid-rs-renderer` (default-features = false) for parse. Both are only needed if the mermaid feature is enabled (e.g. `[features] mermaid = ["mermaid-builder", "mermaid-rs-renderer"]`).

---

## 5. Dependencies

- **mermaid-builder** – Build flowchart syntax (nodes, edges, subgraphs). No parser. [crates.io](https://crates.io/crates/mermaid-builder)
- **mermaid-rs-renderer** – Parse Mermaid (including flowchart) to an IR. We use only the parse step; no rendering. Use `default-features = false` to avoid CLI and PNG. [crates.io](https://crates.io/crates/mermaid-rs-renderer)

We do **not** depend on a single AST: the blueprint (and StreamWeave’s `Graph`/`GraphBuilder`) is the bridge between the two crates.

---

## 6. Limitations and risks

- **Two ASTs:** We maintain two code paths (parse vs export) and the convention document. Changes to the convention must be reflected in both paths and in tests.
- **mermaid-rs-renderer IR:** The crate’s IR is built for layout/rendering. We rely on it preserving node ids, edge endpoints, edge labels, and subgraph membership. If a future version drops or changes any of these, our parser may need updates.
- **Node types:** Imported graphs are topology + metadata only until we attach real nodes (registry or placeholders). We do not infer Rust node types from the diagram.
- **Port names:** Port names are arbitrary strings in StreamWeave; we must escape them in edge labels so that `->` and other reserved characters don’t break the convention. Define a simple escaping scheme (e.g. `\->` or URL-encoding) in Phase 1.

---

## 7. Success criteria

- We can **export** a StreamWeave graph (or blueprint) to a `.mmd` string that conforms to our convention and can be rendered by standard Mermaid tools.
- We can **parse** a `.mmd` file that conforms to our convention into a graph blueprint and, with a registry or placeholders, into a runnable `Graph`.
- Roundtrip (parse → export → parse) preserves structure and metadata up to the defined convention.
- The implementation plan and convention live in `docs/` (and optionally in code under `src/mermaid/`) and are the single reference for future work (e.g. IDE support or sidecar tooling).
