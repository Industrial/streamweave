# Progress Tracking (Watermarks)

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §9.

**Dependencies:** Logical timestamps (implemented).

**Implementation status:** See [IMPLEMENTATION-STATUS.md](IMPLEMENTATION-STATUS.md). Complete: CompletedFrontier, ProgressHandle, execute_with_progress, execute_with_progress_per_sink, WatermarkInjectorNode.

---

## Quick start / Example

Use **`execute_with_progress`** instead of `run_dataflow` when you need to observe the completed frontier (watermark) at the sink:

```rust
use streamweave::graph::Graph;
use streamweave::time::LogicalTime;

let mut graph = Graph::new("g".to_string());
// ... add nodes, expose input/output, connect_timestamped_input_channel ...

let (progress, run) = graph.execute_with_progress();
// Drive input: send Timestamped items, then advance_to(LogicalTime::new(t)) when no more data with time < t.

// Poll progress (e.g. in a loop or after sending a batch):
while progress.less_than(LogicalTime::new(5)) {
    tokio::task::yield_now().await;
}
let completed = progress.frontier(); // minimum completed logical time at the sink
```

**WatermarkInjectorNode** turns a timestamped (or event-time) stream into a stream of `StreamMessage::Data` and `StreamMessage::Watermark`; use it before event-time window nodes. See [event-time-semantics.md](event-time-semantics.md) for event time. **Full runnable example:** `cargo run --example event_time_watermark` (execute_with_progress, WatermarkInjectorNode, frontier). See [examples/event_time_watermark.rs](../examples/event_time_watermark.rs).

---

## 1. Objective and rationale

**Objective:** The runtime (or operators) can answer: “For this input/output, what is the **minimum logical timestamp** that has been completed (e.g. all data with time ≤ T has been processed)?” That is the “progress” or “watermark.” It enables safe closing of event-time windows and correct flushing.

**Why it matters:**

- Without progress, you cannot close windows in event-time correctly (you might close too early or never).
- Required for event-time processing, backpressure, and scheduling (e.g. “do not run round R+1 until round R is done”).

---

## 2. Current state in StreamWeave (what is done)

- **LogicalTime, CompletedFrontier, ProgressHandle:** Implemented in `src/time.rs`.
- **Execution wrapper:** `run_dataflow` accepts `Option<Arc<CompletedFrontier>>`; when set, items on edges carry logical time and the frontier is advanced when items reach **exposed outputs** (sinks).
- **execute_with_progress():** Returns a `ProgressHandle`; callers can use `less_than(t)`, `less_equal(t)`, `frontier()`.
- **Semantics:** “Completed frontier at the sink” = minimum logical time such that all items with time ≤ that value have been delivered to that sink (single-worker, single-graph).

---

## 3. What remains to be done

### 3.1 Per-node progress (optional)

**Today:** Progress is updated only at **sinks** (exposed outputs). Internal nodes do not expose “minimum timestamp I have completed.”

**Enhancement:** For each node (or for a subset), maintain a “completed frontier” that advances as the node consumes and propagates items. This would allow:

- Downstream to know “this node has finished up to time T.”
- Multi-input nodes to define progress as the minimum over input frontiers (e.g. “I have completed up to min(frontier_A, frontier_B)”).

**Design:** Each node could have an optional `Arc<CompletedFrontier>` that it updates when it has processed and forwarded all items with time ≤ T. The execution layer would need to pass a frontier per node and update it when the node’s output for that time has been sent. This is more invasive; defer until needed for event-time windowing or cyclic rounds.

### 3.2 Graph-level progress (multi-sink)

**Today:** Single frontier for the whole graph (one `CompletedFrontier` in `execute_with_progress`), updated when items reach **any** exposed output.

**Enhancement:** If there are multiple sinks, “graph progress” could be defined as the **minimum** over the per-sink frontiers (so that “progress = T” means “all sinks have completed up to T”). Currently all exposed outputs share the same frontier; if we had one frontier per output port, we could take the minimum and expose that as the graph’s progress.

**Design:** Optional: maintain `HashMap<output_port, CompletedFrontier>` and expose `graph_progress() = min(frontiers)`. Useful when different sinks have different rates.

### 3.3 Watermarks as messages

**Today:** Progress is updated implicitly by the runtime when items reach sinks. There is no explicit “watermark” message that flows on a stream.

**Enhancement:** Allow **watermark messages** (e.g. “progress is now T”) to be injected by sources or a coordinator. Operators that buffer by time (e.g. event-time windows) can then:

- Receive a watermark message.
- Advance their view of “completed time” to T.
- Flush or close windows that end before T.

**Design:**

- Introduce a **control message** type, e.g. `StreamMessage<T> = Data(Timestamped<T>) | Watermark(LogicalTime)`.
- Sources (or a dedicated “watermark injector” node) can send `Watermark(T)` when they know no more data with time < T will arrive.
- Window nodes (and other stateful nodes that need progress) consume both data and watermarks; on `Watermark(T)`, they update their internal “completed frontier” and may emit window results for windows that end before T.
- The existing `CompletedFrontier` can still be updated by the runtime at sinks; watermark messages are an **additional** way to propagate progress along the graph (useful when not all progress comes from “item reached sink”).

### 3.4 Distributed progress (later)

When the graph is distributed (multiple workers), progress is the **minimum** over all workers’ progress (or per-partition progress). This requires coordination (e.g. a central aggregator or a distributed protocol). Defer until distribution exists.

---

## 4. Detailed design: watermarks as messages

### 4.1 Type

```rust
pub enum StreamMessage<T> {
    Data(Timestamped<T>),
    Watermark(LogicalTime),
}
```

Channels that carry “data or watermark” use `StreamMessage<Payload>`. Nodes that only care about data can strip watermarks; nodes that need progress (e.g. window) consume both.

### 4.2 Source contract

- **Event-time source:** May emit `Watermark(T)` when it knows no more data with event time < T will be produced. E.g. Kafka source with periodic “current max timestamp seen minus allowed lateness.”
- **Synthetic source:** May emit watermarks based on a timer or on “no data for N ms.”

### 4.3 Window node behavior

- Buffers items by window (event time).
- On `Watermark(T)`: mark “completed time = T”; for each window that ends before T and has not yet been emitted, compute aggregate and emit, then drop window state.
- On `Data(ts)`: add to appropriate window(s); do not close windows until watermark.

### 4.4 Backpressure and scheduling

- Progress can be used for **scheduling**: e.g. “do not send more data to this node until its progress has advanced” (to avoid unbounded buffering). Optional; can be added later.

---

## 5. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | Document current progress semantics (sink frontier, single graph) and what is missing (per-node, watermarks as messages). **Done:** See README § Progress tracking, this doc. |
| **2** | (Optional) Per-sink frontier and graph-level progress = min(sink frontiers). **Done:** `execute_with_progress_per_sink()`, `ProgressHandle::from_frontiers()`. |
| **3** | Introduce `StreamMessage<T> = Data | Watermark` and a path for “stream with watermarks.” |
| **4** | Add watermark injection at sources (or a watermark node) and consumption in window nodes. **Done:** `WatermarkInjectorNode` in `src/nodes/stream/watermark_injector_node.rs`. Accepts Timestamped, StreamMessage, or payloads with `event_timestamp`; outputs `StreamMessage`; emits `Watermark(max_time)` on EOS. `TumblingEventTimeWindowNode` consumes both Data and Watermark. Pipeline: EventTimeExtractorNode → WatermarkInjectorNode → TumblingEventTimeWindowNode. |
| **5** | (With distribution) Distributed progress aggregation. |

---

## 6. References

- Gap analysis §9 (Progress tracking).
- [logical-timestamps-timely-and-streamweave.md](logical-timestamps-timely-and-streamweave.md), [event-time-semantics.md](event-time-semantics.md), [windowing.md](windowing.md).
- `src/time.rs`: `CompletedFrontier`, `ProgressHandle`.
