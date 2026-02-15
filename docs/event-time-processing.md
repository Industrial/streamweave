# Event-Time Processing (End-to-End)

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §12.

**Dependencies:** Event-time semantics, Progress tracking, Windowing (event-time).

**Implementation status:** See [IMPLEMENTATION-STATUS.md](IMPLEMENTATION-STATUS.md). Complete: event-time semantics, watermarks, event-time windows, LateDataPolicy (Drop, SideOutput).

---

## 1. Objective and rationale

**Objective:** End-to-end processing where **ordering**, **windowing**, and **progress** are based on **event time**, with explicit handling of **late data** (drop, buffer, or side output). This is what users mean by “correct streaming with late data.”

**Why it matters:**

- Combines event-time semantics, logical timestamps, progress/watermarks, and event-time windows into one coherent model.
- Enables correct analytics when events arrive out of order or late (e.g. mobile offline, retries, backfill).

---

## 2. Scope of this document

This document does **not** introduce new primitives; it describes the **integration** of:

- [Event-time semantics](event-time-semantics.md): events carry event time; sources and nodes use it.
- [Progress tracking](progress-tracking.md): watermarks (progress) indicate “no more data before T.”
- [Windowing](windowing.md): event-time windows (tumbling, sliding, session) that close when the watermark passes.

Plus one cross-cutting concern: **late-data policy**.

---

## 3. End-to-end flow

1. **Sources:** Produce (or annotate) events with **event time**. Optionally emit **watermarks** (“no more data with event time < T”).
2. **Channels:** Carry `StreamMessage<T> = Data(Timestamped<T>) | Watermark(LogicalTime)` so that progress propagates.
3. **Operators:** Use event time for ordering and windowing; use watermarks to close windows and flush state.
4. **Late data:** When an event arrives with event_time T and watermark has already passed T, apply the configured **late-data policy** (drop, allowed lateness, or side output).
5. **Sinks:** Receive window results (or stream output) in event-time order (or watermark order); no special sink change beyond consuming the stream.

---

## 4. Late-data policy (detailed)

### 4.1 Drop

- **Behavior:** Discard any event whose event time is before the current watermark (or before “closed” windows).
- **Pros:** Simple; no extra state. **Cons:** Data loss; not suitable when late data must be counted.

### 4.2 Allowed lateness

- **Behavior:** Define a **lateness bound** (e.g. 5 minutes). A window is not considered “closed” until watermark > window_end + lateness. Events that arrive within that bound are added to the window; the window result may be **updated** (re-emitted) or **retracted + re-emitted** (if using differential).
- **State:** Keep window state for (window_end + lateness) after watermark passes window_end; then discard.
- **Pros:** Captures most late data. **Cons:** More state and possible duplicate/update emissions.

### 4.3 Side output

- **Behavior:** Send late events to a **separate stream** (e.g. “late” port). Downstream can log, store, or aggregate them separately.
- **Pros:** No data loss; clear separation. **Cons:** User must handle the late stream.

### 4.4 Configuration

- **Per-node or per-graph:** e.g. `LateDataPolicy::Drop | AllowLateness(Duration) | SideOutput`. Default: Drop (safe and simple).

---

## 5. Implementation checklist

| Component | Requirement |
|-----------|--------------|
| **Sources** | Emit event time (and optionally watermarks). |
| **Stream type** | `StreamMessage<T>` or equivalent so watermarks flow. |
| **Window nodes** | Use event time for assignment; close on watermark; apply late-data policy. |
| **Progress** | Watermarks propagated from sources (or derived); graph-level progress = min over relevant frontiers. |
| **Docs** | Document “event-time processing” as the combination of the above; document late-data policy options. |

---

## 6. How to wire an event-time pipeline

### 6.1 Steps

1. **Provide timestamped input.** Use `graph.connect_timestamped_input_channel(port, receiver)` where the receiver yields `Timestamped<Arc<dyn Any>>` with event time in `time`. Or use `EventTimeExtractorNode` to wrap a payload stream and extract event time from each item.

2. **Use progress-aware execution.** Call `graph.execute_with_progress()` (or `execute_with_progress_per_sink()`) so the runtime propagates watermarks from sources to sinks and advances the frontier.

3. **Add event-time window nodes.** Insert `TumblingEventTimeWindowNode`, `SlidingEventTimeWindowNode`, or `SessionEventTimeWindowNode` in the path. These require `StreamMessage` (Data/Watermark) input; when using the timestamped path, the graph supplies this automatically.

4. **Configure late-data policy.** Each event-time window node supports `with_late_data_policy(policy)`:
   - `LateDataPolicy::Drop` (default): discard events whose event time is before the watermark.
   - `LateDataPolicy::SideOutput`: send late events to the `"late"` output port for separate handling.

5. **Connect outputs.** Exposed output ports receive window results (or stream output). The `"late"` port, when using SideOutput, carries late events only.

### 6.2 Example sketch

```rust
// Connect timestamped input (event time from your source)
graph.connect_timestamped_input_channel("events", timestamped_rx)?;

// Or use EventTimeExtractorNode to add event time from payload
let extractor = EventTimeExtractorNode::new("extract", event_time_from_map());
graph.add_node("extract", Box::new(extractor))?;
// ... wire extractor output to downstream

// Event-time window with side output for late data
let window = TumblingEventTimeWindowNode::new("win", window_size)
    .with_late_data_policy(LateDataPolicy::SideOutput);
graph.add_node("win", Box::new(window))?;
// Connect "late" port to a sink or logger

graph.execute_with_progress().await?;
```

---

## 7. Testing

- **Out-of-order:** Feed events in wrong order (e.g. time 3, 1, 2); assert window results are correct when watermark passes. **Done:** `test_event_time_window_out_of_order`, `test_sliding_window_out_of_order`, `test_session_window_out_of_order`.
- **Late data:** Feed event with time T after watermark has passed T; assert drop / allowed-lateness update / side-output emission per policy. **Done:** `test_event_time_window_late_data_dropped`, `test_event_time_window_late_data_side_output`.
- **Backfill:** Replay a batch of old events; assert they are assigned to correct windows and watermarks advance correctly.

---

## 8. References

- Gap analysis §12 (Event-time processing).
- [event-time-semantics.md](event-time-semantics.md), [progress-tracking.md](progress-tracking.md), [windowing.md](windowing.md).
