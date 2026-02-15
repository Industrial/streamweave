# Event-Time Semantics

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §8.

**Dependencies:** Logical timestamps (implemented).

**Implementation status:** See [IMPLEMENTATION-STATUS.md](IMPLEMENTATION-STATUS.md). Complete: HasEventTime, EventTimeExtractorNode, connect_timestamped_input_channel.

---

## 1. Objective and rationale

**Objective:** Use the time **attached to the event** (e.g. “when the click happened”) as the basis for ordering, windowing, and progress, rather than **processing time** (“when we saw the event”). This matches user intuition and is required for correct analytics with late or out-of-order data.

**Why it matters:**

- Processing-time-only semantics are wrong for late data and backfill.
- Event-time is the standard in modern stream systems (Flink, Beam, Kafka Streams).
- Enables correct “per hour” or “per day” aggregations when events arrive out of order.

---

## 2. Current state in StreamWeave

- **TimestampNode:** Adds **processing time** (e.g. `SystemTime::now()`). No first-class event-time field or semantics.
- **Windowing:** Count-based (fixed-size batches); no time-based or event-time windows.
- **Logical time:** Implemented as `LogicalTime` and used for progress; can be **reused or mapped** for event time when the source provides it (e.g. event timestamp in ms as `LogicalTime`).

---

## 3. Event time vs processing time

| Aspect | Processing time | Event time |
|--------|------------------|------------|
| **Definition** | Time when the system processes the event | Time when the event actually occurred (from payload or metadata) |
| **Ordering** | Order of arrival | Order by event time (with possible late data) |
| **Windowing** | “Current” window by wall clock | Window by event time; windows close when watermark says “no more data for that window” |
| **Late data** | N/A (everything is “on time”) | Must define policy: drop, buffer, or side output |

---

## 4. Detailed design

### 4.1 Event time as first-class

**Convention or trait:** Define how “event time” is carried and extracted.

**Option A – Convention:** Events that carry event time have a well-known field or type, e.g.:

- A trait `EventTimeExtractor` or `HasEventTime`: `fn event_time_ms(&self) -> u64` (or `Option<u64>` for events that may not have a time).
- Payload types that implement this can be used in event-time windowing and watermarking.

**Option B – Envelope:** Wrap payloads in an envelope that includes event time, e.g. `EventTimeEnvelope<T> { payload: T, event_time: LogicalTime }`. The runtime and operators use `event_time` for ordering and windows. This aligns with `Timestamped<T>` if we interpret `time` as event time when in event-time mode.

**Recommendation:** Use **LogicalTime** (or a type alias `EventTime = LogicalTime`) for event time. Sources that have a real event timestamp (e.g. from Kafka, or from a field in the payload) should set that as the logical time when feeding the graph (e.g. via a node that extracts event time and forwards `Timestamped { payload, time: event_time }`). So “event time as first-class” = use the existing timestamped path with **event time** as the logical time, not a synthetic counter.

### 4.2 Source responsibilities

- **Event-time source:** A source (or an adapter node) that:
  - Reads payloads that have an event timestamp (e.g. `created_at` in the record).
  - Emits `Timestamped { payload, time: LogicalTime::new(event_time_ms) }` (or equivalent).
  - Optionally emits **watermarks** (see [progress-tracking.md](progress-tracking.md)): “no more data with event time < T will arrive,” so that downstream can close windows.

- **Processing-time source:** Keeps existing behavior (e.g. `TimestampNode` that sets time = now). Document as “processing time” for backward compatibility.

### 4.3 Event-time windowing

- **Windowing** (see [windowing.md](windowing.md)): Assign items to windows by **event time**. E.g. tumbling window [0, 3600) ms: all events with `event_time` in that range go into the same window.
- **Closing windows:** A window is “closed” when the **watermark** (progress) indicates that no more data with event time in that window will arrive. Then the window can be emitted (e.g. aggregate result).
- **Late data policy:** If an event arrives with event time T but the watermark has already passed T (e.g. we already closed the window for T), apply a policy: **drop**, **buffer** (allowed lateness), or **side output** to a “late” stream.

### 4.4 API surface

- **Extract event time:** Provide a helper or trait so that user payloads can expose event time (e.g. `impl HasEventTime for MyEvent`).
- **Event-time input:** When connecting external input, allow a **timestamped input** where each item is `(payload, event_time)`. The execution layer already supports timestamped path; ensure that when the user provides event time, it is used as the logical time (no overwrite with counter).
- **Processing-time node:** Keep `TimestampNode` (or equivalent) documented as “processing time”; add **EventTimeExtractorNode** or “event-time input” that uses payload event time.

### 4.5 Pragmatics

- Keep processing-time nodes for backward compatibility.
- Add event-time as an **explicit path**: e.g. “use event-time sources and event-time window nodes; progress/watermarks drive window closing.”
- Document when to use event time vs processing time.

---

## 5. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | Document event time vs processing time; define convention or trait for “event time” on payloads. **Done:** See `time::HasEventTime`, [README § Time Semantics](../README.md#-time-semantics). |
| **2** | Ensure timestamped execution path can use **external** event time (user-provided) instead of only internal counter. **Done:** `connect_timestamped_input_channel(port, Receiver<Timestamped<Arc<dyn Any>>>)`; with `execute_with_progress`, user time is used. |
| **3** | Add event-time window nodes (depends on [windowing.md](windowing.md) and [progress-tracking.md](progress-tracking.md)). **Partial:** EventTimeExtractorNode added (extracts event time from payloads, adds `event_timestamp` field). |
| **4** | Add watermark propagation (progress) so that windows can close; add late-data policy (drop / buffer / side output). |

---

## 6. References

- Gap analysis §8 (Event-time semantics).
- [logical-timestamps-timely-and-streamweave.md](logical-timestamps-timely-and-streamweave.md), [progress-tracking.md](progress-tracking.md), [windowing.md](windowing.md), [event-time-processing.md](event-time-processing.md).
