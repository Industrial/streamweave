# Windowing

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §11.

**Dependencies:** None for count-based and processing-time. Event-time semantics and Progress tracking for event-time windows.

---

## 1. Objective and rationale

**Objective:** Group events into **finite windows** (tumbling, sliding, session, or custom) for aggregation or join. Windows can be defined by **count**, **processing time**, or **event time**.

**Why it matters:**

- Most stream analytics (rates, counts per minute, sessions, etc.) are expressed as windows.
- Count-only windows are limiting; time-based (especially event-time) windows are expected in mature systems.

---

## 2. Current state in StreamWeave

- **WindowNode:** Exists and is **count-based** (fixed number of items per batch). No time-based or event-time windows.
- **No tumbling/sliding/session** by time; no watermark-based window closing.

---

## 3. Window types

| Type | Description | Key parameters |
|------|-------------|-----------------|
| **Tumbling** | Fixed-size, non-overlapping intervals | Size (e.g. 1 hour). |
| **Sliding** | Fixed size, smaller step (overlapping) | Size, slide (e.g. size 1h, slide 15m). |
| **Session** | Gap-based; new window after idle period | Gap duration (e.g. 30 min). |
| **Count** | Fixed number of items per window | Count (existing). |

---

## 4. Time domains

### 4.1 Count-based (existing)

- **Semantics:** Every N items form one window. Order is arrival order (or logical time if available).
- **Keep:** Document as “count window”; no change required for existing behavior.

### 4.2 Processing-time windows

- **Semantics:** Emit a new window every N ms (wall clock). Assign each item to the “current” window at the time it is processed.
- **Determinism:** Deterministic per run only if scheduling is fixed; otherwise “current window” depends on when the item was processed.
- **Implementation:** Buffer items; every `window_size` ms (or on timer), close the current window (emit aggregate), start a new window. No watermark needed.

### 4.3 Event-time windows

- **Semantics:** Assign each item to a window by its **event time** (logical time or extracted event timestamp). Windows are intervals of event time (e.g. [0, 3600), [3600, 7200)).
- **Closing:** A window is “closed” when the **watermark** (progress) shows that no more data with event time in that window will arrive. Then emit the aggregate for that window.
- **Requires:** Event-time semantics, progress/watermarks (see [event-time-semantics.md](event-time-semantics.md), [progress-tracking.md](progress-tracking.md)).
- **Late data:** Events that arrive after the window is closed need a policy: drop, buffer (allowed lateness), or side output.

---

## 5. Detailed design

### 5.1 Tumbling (event-time)

- **Parameters:** `window_size: Duration` (e.g. 3600s). Window boundaries: [0, size), [size, 2*size), ...
- **Assignment:** Item with event time T goes to window `floor(T / size) * size`.
- **State:** Per-window buffer: list or aggregate (e.g. sum, count). On **Watermark(T)** (or progress T): close all windows that end before T (i.e. window_end <= T); emit their aggregates; drop state.
- **Output:** Stream of (window_start, window_end, aggregate) or (window_id, aggregate).

### 5.2 Sliding (event-time)

- **Parameters:** `size: Duration`, `slide: Duration` (e.g. size 1h, slide 15m). Windows: [0, 1h), [15m, 1h15m), [30m, 1h30m), ...
- **Assignment:** Item with event time T belongs to all windows that contain T (i.e. window_start <= T < window_end). So one item may be in multiple windows.
- **State:** More expensive (many overlapping windows). Emit when watermark passes window_end.
- **Optimization:** Share state across windows (e.g. segment tree or incremental aggregation) to avoid redundant work.

### 5.3 Session (event-time)

- **Parameters:** `gap: Duration`. A new window starts when an event arrives more than `gap` after the previous event (per key, if keyed).
- **Assignment:** Events are assigned to the “current” session for that key; if event_time - last_event_time > gap, start a new session.
- **State:** Per-key: (current_session_start, last_event_time, aggregate). On watermark, close sessions that have been idle (last_event_time + gap < watermark).
- **Output:** (session_start, session_end, key, aggregate).

### 5.4 Processing-time tumbling (simple)

- **Parameters:** `window_size: Duration`. Every `window_size` wall-clock time, close the current window and emit.
- **State:** Buffer items for “current” window; on timer, emit and clear. No watermark; no late data.

### 5.5 Node API

**New nodes (or extend WindowNode):**

- `TumblingEventTimeWindowNode { size: Duration }` – requires event-time input and watermark stream (or progress handle).
- `SlidingEventTimeWindowNode { size: Duration, slide: Duration }`.
- `SessionWindowNode { gap: Duration }` (keyed).
- `TumblingProcessingTimeWindowNode { size: Duration }` – timer-based.

Input: stream of `Timestamped<T>` (or `StreamMessage<T>` with watermarks). Output: stream of (window, aggregate) or similar.

### 5.6 Late data policy (event-time)

When an event arrives with event_time T but watermark has already passed T:

- **Drop:** Discard the event (simplest).
- **Allowed lateness:** Keep window state for T + allowed_lateness; if event arrives within that, add to window and re-emit (or update) the window result. Requires “retract” or “update” semantics (see differential).
- **Side output:** Send late events to a separate “late” stream for separate handling.

---

## 6. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | Document count window (existing); add processing-time tumbling window node (timer-based). |
| **2** | Implement tumbling event-time window (buffer by window, close on watermark). Requires progress/watermark integration. |
| **3** | Add sliding and session windows (event-time). |
| **4** | Add late-data policy (configurable: drop, allowed lateness, side output). |

---

## 7. References

- Gap analysis §11 (Windowing).
- [event-time-semantics.md](event-time-semantics.md), [progress-tracking.md](progress-tracking.md), [event-time-processing.md](event-time-processing.md).
- Flink/Beam: window semantics and allowed lateness.
