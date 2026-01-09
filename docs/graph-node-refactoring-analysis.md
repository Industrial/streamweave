# Graph Node Refactoring Analysis

## Overview

This document analyzes the relationship between graph nodes (`src/graph/nodes/`) and core streamweave components (`src/transformers/`, `src/producers/`, `src/consumers/`). The goal is to identify opportunities to refactor graph nodes to use existing transformers/producers/consumers, reducing code duplication and ensuring consistency.

## Key Finding: GroupBy Duplication

**Current State:**
- `src/graph/nodes/group_by.rs` implements `GroupBy` as a standalone transformer
- `src/transformers/group_by_transformer.rs` implements `GroupByTransformer` with similar functionality
- Both group items by a key function and output `(K, Vec<T>)`
- **The graph node could easily use the transformer!**

**Differences:**
- `GroupByTransformer` sorts groups by key for deterministic output (line 117-118)
- `GroupBy` (graph node) doesn't sort, emits groups in hash map order
- `GroupByTransformer` requires `K: Ord` in addition to `Hash + Eq`
- `GroupBy` uses `Arc<dyn Fn(&T) -> K>` while `GroupByTransformer` uses generic `F: Fn(&T) -> K`

**Recommendation:** Refactor `GroupBy` to wrap `GroupByTransformer` internally, or make `GroupByTransformer` the canonical implementation and have `GroupBy` delegate to it.

---

## Graph Nodes That SHOULD Use StreamWeave Components

These graph nodes duplicate functionality that already exists in transformers/producers/consumers and should be refactored to use them:

### 1. **GroupBy** → Use `GroupByTransformer`
- **Location:** `src/graph/nodes/group_by.rs`
- **Target:** `src/transformers/group_by_transformer.rs`
- **Rationale:** Nearly identical functionality. The transformer has better features (sorted output).
- **Action:** Refactor `GroupBy` to wrap `GroupByTransformer` or delegate to it.

### 2. **Timeout** → Use `TimeoutTransformer`
- **Location:** `src/graph/nodes/timeout.rs`
- **Target:** `src/transformers/timeout_transformer.rs`
- **Rationale:** Both apply timeouts to stream processing. The transformer is more feature-complete.
- **Action:** Refactor `Timeout` to wrap `TimeoutTransformer`.

### 3. **RoundRobinRouter** → Could use `RoundRobinTransformer` (with adapter)
- **Location:** `src/graph/nodes/round_robin_router.rs`
- **Target:** `src/transformers/round_robin_transformer.rs`
- **Rationale:** Similar round-robin distribution logic, but different interfaces:
  - Router: Implements `OutputRouter` trait, routes to multiple output ports
  - Transformer: Single input/output, distributes to multiple consumers via config
- **Action:** Consider creating an adapter or refactoring to share core logic.

### 4. **MergeRouter** → Could use `MergeTransformer` (with adapter)
- **Location:** `src/graph/nodes/merge_router.rs`
- **Target:** `src/transformers/merge_transformer.rs`
- **Rationale:** Both merge multiple streams, but:
  - Router: Implements `InputRouter` trait, merges multiple input ports
  - Transformer: Takes multiple streams via `add_stream()`, single output
- **Action:** Consider creating an adapter or refactoring to share core logic.

### 5. **KeyBasedRouter** → Could use `PartitionTransformer` (with adapter)
- **Location:** `src/graph/nodes/key_based_router.rs`
- **Target:** `src/transformers/partition_transformer.rs`
- **Rationale:** Both partition/route based on keys/predicates, but:
  - Router: Routes to N output ports based on key hash
  - Transformer: Partitions into 2 streams (true/false predicate)
- **Action:** Consider generalizing `PartitionTransformer` to support N-way partitioning, or create adapter.

### 6. **If Router** → Could use `PartitionTransformer` (with adapter)
- **Location:** `src/graph/nodes/if_router.rs`
- **Target:** `src/transformers/partition_transformer.rs`
- **Rationale:** `If` routes based on predicate (true → port 0, false → port 1), which is exactly what `PartitionTransformer` does.
- **Action:** Refactor `If` to use `PartitionTransformer` internally, adapting the router interface.

### 7. **Aggregate** → Could use `ReduceTransformer` (with specialization)
- **Location:** `src/graph/nodes/aggregate.rs`
- **Target:** `src/transformers/reduce_transformer.rs`
- **Rationale:** Both aggregate/reduce items, but:
  - `Aggregate`: Specialized aggregators (Sum, Count, Min, Max), optional windowing
  - `ReduceTransformer`: Generic reducer function, no windowing
- **Action:** Consider making `Aggregate` use `ReduceTransformer` internally, or vice versa.

### 8. **Delay** → Could be a simple transformer wrapper
- **Location:** `src/graph/nodes/delay.rs`
- **Target:** Could create `DelayTransformer` or use `RateLimitTransformer` with rate=1
- **Rationale:** Simple delay functionality could be a transformer.
- **Action:** Consider creating `DelayTransformer` in transformers module, or use existing rate limiting.

---

## Graph Nodes That Are Unique (Control Flow)

These graph nodes provide unique control flow functionality that doesn't have direct equivalents in transformers:

1. **ErrorBranch** - Routes based on error conditions (unique error handling)
2. **ForEach** - Iterates over collections (unique collection processing)
3. **Join** - Joins multiple streams on keys (unique join semantics)
4. **Match** - Pattern matching with multiple patterns (unique pattern matching)
5. **Synchronize** - Synchronizes multiple input streams (unique synchronization)
6. **Variables** - Graph-level variable management (unique state management)
7. **While** - Loop constructs with conditions (unique control flow)
8. **BroadcastRouter** - Broadcasts to all output ports (unique fan-out pattern)

**Note:** These should remain as graph-specific nodes, but could potentially use transformers internally for some operations.

---

## StreamWeave Components That COULD Get Graph Nodes

These transformers/producers/consumers already work via `TransformerNode::from_transformer()`, `ProducerNode::from_producer()`, and `ConsumerNode::from_consumer()`, but some might benefit from specialized graph node wrappers:

### Transformers That Already Work (via TransformerNode):
- ✅ All transformers work via `TransformerNode::from_transformer()`
- No additional graph nodes needed for basic usage

### Transformers That Might Benefit from Specialized Wrappers:

1. **RouterTransformer** → Could have a dedicated `RouterNode`
   - **Location:** `src/transformers/router_transformer.rs`
   - **Rationale:** Already works via `TransformerNode`, but a specialized router node might provide better graph integration

2. **PartitionTransformer** → Could have a dedicated `PartitionNode`
   - **Location:** `src/transformers/partition_transformer.rs`
   - **Rationale:** Two-output transformer could benefit from explicit two-port graph node

3. **SplitTransformer** → Could have a dedicated `SplitNode`
   - **Location:** `src/transformers/split_transformer.rs`
   - **Rationale:** Splits streams into groups, might benefit from graph-specific handling

4. **WindowTransformer** / **TimeWindowTransformer** → Could have dedicated nodes
   - **Location:** `src/transformers/window_transformer.rs`, `src/transformers/time_window_transformer.rs`
   - **Rationale:** Windowing is a common graph pattern, specialized nodes might help

5. **BatchTransformer** → Could have a dedicated `BatchNode`
   - **Location:** `src/transformers/batch_transformer.rs`
   - **Rationale:** Batching is common in graphs, specialized node might help

### Producers That Already Work (via ProducerNode):
- ✅ All producers work via `ProducerNode::from_producer()`
- No additional graph nodes needed

### Consumers That Already Work (via ConsumerNode):
- ✅ All consumers work via `ConsumerNode::from_consumer()`
- No additional graph nodes needed

---

## Summary Tables

### Graph Nodes → Should Use Transformers

| Graph Node | Transformer | Priority | Notes |
|------------|------------|----------|-------|
| `GroupBy` | `GroupByTransformer` | **HIGH** | Nearly identical, transformer has sorting |
| `Timeout` | `TimeoutTransformer` | **HIGH** | Identical functionality |
| `If` | `PartitionTransformer` | **MEDIUM** | Same predicate-based routing, needs adapter |
| `RoundRobinRouter` | `RoundRobinTransformer` | **LOW** | Different interfaces (router vs transformer) |
| `MergeRouter` | `MergeTransformer` | **LOW** | Different interfaces (router vs transformer) |
| `KeyBasedRouter` | `PartitionTransformer` | **LOW** | Different (N-way vs 2-way), needs generalization |
| `Aggregate` | `ReduceTransformer` | **LOW** | Different (specialized vs generic), could share logic |
| `Delay` | (Create `DelayTransformer`) | **LOW** | Simple, could be a transformer |

### Transformers → Could Get Graph Nodes

| Transformer | Graph Node | Priority | Notes |
|-------------|------------|----------|-------|
| `PartitionTransformer` | `PartitionNode` | **MEDIUM** | Two-output transformer, explicit ports helpful |
| `RouterTransformer` | `RouterNode` | **LOW** | Already works, specialized node might help |
| `SplitTransformer` | `SplitNode` | **LOW** | Could benefit from graph-specific handling |
| `WindowTransformer` | `WindowNode` | **LOW** | Common pattern, specialized node might help |
| `BatchTransformer` | `BatchNode` | **LOW** | Common pattern, specialized node might help |

---

## Recommended Refactoring Order

1. **Phase 1: High Priority (Easy Wins)**
   - Refactor `GroupBy` to use `GroupByTransformer`
   - Refactor `Timeout` to use `TimeoutTransformer`

2. **Phase 2: Medium Priority (Adapters Needed)**
   - Refactor `If` to use `PartitionTransformer` (create router adapter)
   - Create `PartitionNode` wrapper for `PartitionTransformer`

3. **Phase 3: Low Priority (Architecture Decisions)**
   - Evaluate router vs transformer interfaces for `RoundRobinRouter`/`RoundRobinTransformer`
   - Evaluate router vs transformer interfaces for `MergeRouter`/`MergeTransformer`
   - Consider generalizing `PartitionTransformer` for N-way partitioning
   - Consider creating `DelayTransformer` in transformers module

---

## Implementation Notes

### How Graph Nodes Can Use Transformers

Graph nodes can use transformers in two ways:

1. **Direct Delegation:** The graph node wraps the transformer and delegates all work to it:
   ```rust
   pub struct GroupBy<T, K> {
       transformer: GroupByTransformer<...>,
   }
   ```

2. **Adapter Pattern:** The graph node adapts the transformer's interface to the graph's router/transformer interface:
   ```rust
   pub struct If<O> {
       partition: PartitionTransformer<...>,
   }
   // Adapts PartitionTransformer's two-output stream to router's port-based interface
   ```

### Key Differences to Handle

1. **Router Interface:** Graph routers implement `OutputRouter`/`InputRouter` traits with port-based routing, while transformers have single input/output streams. Adapters needed.

2. **Message Handling:** Graph nodes handle `Message<T>` wrapping/unwrapping, transformers work with raw types. This is already handled by `TransformerNode`, so graph nodes that wrap transformers should preserve this.

3. **Port Management:** Graph nodes have explicit port names and indices, transformers have implicit single input/output. Adapters needed for multi-port scenarios.

