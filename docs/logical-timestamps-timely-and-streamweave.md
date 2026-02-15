# Logical Timestamps: Timely Dataflow’s Model and Application to StreamWeave

**Purpose:** This document explains how **Timely Dataflow** implements logical timestamps, why they are the foundation of its execution model, and how **StreamWeave** can adopt an equivalent approach as its base abstraction. It is the first step in the roadmap described in [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md).

---

## 1. Why timestamps first?

In a dataflow system, data moves between operators without a global notion of “when” or “in what order” things happened. That makes it hard to:

- **Correlate inputs and outputs** — e.g. which bank transactions are reflected in which balance?
- **Order work** — e.g. “sum so far” must be by logical order, not arrival order.
- **Know when work is done** — so we can close windows, flush state, or advance to the next round.
- **Support cycles** — iterative algorithms need rounds; rounds are timestamps.

**Logical timestamps** are the minimal mechanism that fixes this: every piece of data carries a **logical time** (not necessarily wall-clock). That time is the single abstraction used for ordering, progress, and coordination. In Timely, **all streams are timestamped**; there is no “raw” stream type. StreamWeave today has no first-class time; adding it as the base is the right first step.

---

## 2. How Timely Dataflow implements timestamps

### 2.1 What a timestamp is (and isn’t)

- **Logical, not physical.** A timestamp can be a sequence number, a batch id, a `(scope, round)` pair, or a nested structure. It does not have to be “event time” or “processing time”; it is whatever the program uses to order and correlate data.
- **Attached to every record.** There is no separate “timestamp channel”; each message is `(data, timestamp)` (conceptually). Operators see **batches** of records that share the same timestamp.
- **Preserved by default.** Unless an operator explicitly changes timestamps, outputs carry the same timestamp as the inputs that produced them, so input–output correlation is preserved.

### 2.2 The `Timestamp` trait (Rust)

From the timely crate (docs.rs), a type used as a timestamp must implement:

```text
pub trait Timestamp:
    Clone + Eq + PartialOrder + Ord + Debug + Any + ExchangeData
{
    type Summary: PathSummary<Self> + 'static;

    fn minimum() -> Self;
}
```

- **PartialOrder + Ord:** The type is partially ordered (e.g. for nested scopes), and the implementation must satisfy: if `a.less_equal(b)` then `a <= b`. So “logical order” and “comparison” agree.
- **minimum():** A unique minimum element (e.g. `0` for `u64`, or `(Root, 0)` for a product type). Used as the initial capability and as a base for “no data yet.”
- **Summary / PathSummary:** Used to summarize how timestamps are transformed along a path in the dataflow (e.g. across nested scopes or delays). For a flat, single-scope design you can start with a trivial summary.

So: timestamps are **ordered**, have a **minimum**, and can be **summarized along paths**. For StreamWeave’s first version, a **totally ordered** type (e.g. `u64` or a small tuple) with a trivial `PathSummary` is enough.

### 2.3 Example: `(Root, i)`

In Timely’s “hello” example, timestamps are printed as `(Root, 0)`, `(Root, 1)`, etc. So:

- The first component (`Root`) is the “scope” (top-level input).
- The second is a sequence number.
- Data is sent with timestamp `(Root, i)`; the input advances its capability so that progress can be tracked and operators can reason about “all data for round `i` has been seen.”

This gives **batch-by-batch** semantics: each `i` is a logical batch, and the system can track when batch `i` has fully flowed through the graph.

### 2.4 Input and capabilities

Input is provided via an **`InputHandle`**. It has an associated **timestamp** (the “current capability”):

- **send(data)** — Sends data with the handle’s **current** timestamp. Data is moved into the dataflow; the call is non-blocking.
- **advance_to(time)** — Announces: “I will not send any more data with timestamp &lt; `time`.” The argument must be ≥ current time (enforced; panic otherwise). This is the **critical** call that drives progress: it tells the system that earlier timestamps are “closed,” so operators waiting on them can complete and the system can advance.
- **close()** — Consumes the handle and prevents further sends; the system can then conclude that no more input will ever arrive.

So: the **producer** is responsible for (1) attaching a timestamp to data (implicitly, via the handle’s current time) and (2) **advancing** that time so the rest of the system can make progress. Forgetting to call `advance_to` is a common bug and leads to the computation appearing stuck.

**Efficiency:** Progress tracking cost is proportional to the **number of distinct timestamps** introduced. So batching many records under the same timestamp is preferred; using a new timestamp per record is discouraged.

### 2.5 Progress: probes

A **probe** is attached to a point in the dataflow (e.g. after an operator). It does not block; it only answers:

- **less_than(t)** — “Is it still possible that I will see a message with timestamp &lt; `t` at this point?”
- **less_equal(t)** — Same for ≤.

So the **consumer** (or driver) can poll: “Has all data with timestamp &lt; `t` been processed past this point?” If not, it can wait or do other work; if yes, it can advance input time, close windows, or move to the next round. Progress is **passive**: the system reports what has happened; the driver decides how to react (e.g. advance_to, or introduce more data).

### 2.6 Operators and batch-by-timestamp access

Operators can get **batch-level** access to data:

- **inspect_batch(|timestamp, batch| { ... })** — Receives batches of records that share the same timestamp. So an operator can process “all data for time `t`” together, and can implement “sum so far by timestamp order” by buffering and sorting by `t`, or by relying on progress to know when `t` is complete.

So in Timely:

1. **Every** message has a timestamp (no raw stream).
2. **Sources** hold a capability and advance it via `advance_to`.
3. **Probes** report whether a given timestamp can still appear at a point.
4. **Operators** can see batches keyed by timestamp and use progress to know when a timestamp is “done.”

---

## 3. Applying the technique to StreamWeave

### 3.1 Current state

- **Item type:** `Stream<Item = Arc<dyn Any + Send + Sync>>`. No timestamp in the type; no notion of “batch” or “round.”
- **Execution:** The graph connects output streams to input streams and forwards items one-by-one (`stream.next()` → `tx.send(item)`). Order is whatever the runtime and async scheduling produce.
- **Sources:** External input is via channels; there is no “current time” or “advance_to.”
- **Progress:** None. Nothing answers “have we finished all data before time T?”

To align with Timely’s benefits (correlation, ordering, progress, cycles), StreamWeave needs a **first-class logical timestamp** and a **runtime that attaches and advances it**.

### 3.2 Design principles (Timely-inspired)

1. **Timestamps are universal** — Either every item in the “timestamped” execution path carries a logical time, or we wrap legacy streams with a default timestamp so that the rest of the system still sees (data, time).
2. **Sources own capabilities** — Whoever introduces data must be able to “advance” logical time (e.g. “no more data with time &lt; T”), so that progress can be defined.
3. **Progress is observable** — Some mechanism (probe-like or callback) must allow the driver or operators to learn “minimum timestamp completed at this point.”
4. **Batching by time is encouraged** — Use fewer distinct timestamps where possible; treat “timestamp” as “batch” or “round” when that fits the use case.

### 3.3 Proposed abstraction for StreamWeave

**1. Logical time type**

- Introduce a **`LogicalTime`** type (or reuse a type parameter `T: LogicalTime`). For a first version, a **totally ordered** type is enough:
  - **Option A:** `u64` — simple, no extra traits.
  - **Option B:** Small struct, e.g. `pub struct LogicalTime { epoch: u32, seq: u64 }` — allows multiple “epochs” (e.g. restarts) and a sequence within each.
- Require: `Clone + Eq + Ord + Send + Sync + Default` (and `Default::default()` as the minimum). Defer `PathSummary` until nested scopes or delayed timestamps are needed.

**2. Envelope type**

- Every item in the timestamped model is an **envelope** that carries payload + time:
  - `pub struct Timestamped<T> { pub payload: T, pub time: LogicalTime }`
- Or, for type-erased streams: `Stream<Item = Timestamped<Arc<dyn Any + Send + Sync>>>` (or a single type `Timestamped<Payload>` where `Payload` is the existing `Arc<dyn Any + Send + Sync>`).
- **Compatibility:** Existing `Node` API continues to use `Stream<Item = Arc<dyn Any + Send + Sync>>`. A **wrapper** or **adapter** at graph execution time can inject a default timestamp (e.g. `LogicalTime::default()` or a monotonically increasing counter) so that existing graphs run unchanged while the engine still sees timestamped items internally.

**3. Source / input side**

- For **external** input (the analogue of Timely’s `InputHandle`):
  - Provide a **timestamped input handle** that has:
    - `send(payload)` — sends `Timestamped { payload, time: current_time }`.
    - `advance_to(time)` — sets current capability to `time` (monotonic).
    - Optionally `close()`.
  - The graph execution layer uses this to (1) attach timestamps to incoming data and (2) drive progress (see below).
- For **internal** sources (nodes with no inputs): they must produce a stream of `Timestamped<Payload>`. They can use a counter or a provided “clock” to assign times; if they support “rounds,” they call something like `advance_to(round + 1)` when a round is done.

**4. Progress**

- **Single-worker, single-graph:** Maintain a “completed frontier” per sink or per a designated progress point: e.g. the minimum `LogicalTime` such that all items with time ≤ that have been processed past that point. Expose this via:
  - A **probe-like** handle: `probe.less_than(t)` / `less_equal(t)` (non-blocking), or
  - A **stream of progress updates**: e.g. `Stream<Item = LogicalTime>` meaning “progress is now at least T.”
- Implementation: when an item with time `t` is consumed at a sink (or at the progress point), update the frontier (e.g. “minimum completed” = min over all inputs). This requires that the execution layer sees timestamps; hence the envelope or the wrapper.

**5. Graph execution with timestamps**

- **Option A — Wrapper layer:** Keep existing `Node::execute(InputStreams) -> OutputStreams` with `Arc<dyn Any>` streams. In the execution layer, wrap each stream in an adapter that:
  - On read: wraps each `Arc<dyn Any>` in `Timestamped { payload, time }` with a default or injected time.
  - On write: when a node produces `Arc<dyn Any>`, the adapter attaches a time (e.g. from the input batch or a counter) and forwards `Timestamped { .. }` downstream. Progress is updated when those items are consumed.
- **Option B — Native timestamped API:** Add a second execution path (e.g. `execute_timestamped`) where `InputStreams` and `OutputStreams` are `Stream<Item = Timestamped<Payload>>`. Nodes that implement a `TimestampedNode` trait work with batches or timestamped items directly; the rest can be wrapped. This allows a gradual migration.

Recommendation: **start with Option A** (wrapper + default timestamp) so that (1) all existing graphs keep working, (2) the engine can maintain progress and future operators can use it, and (3) we can introduce `TimestampedNode` and Option B later for operators that need batch-by-time or custom time handling.

### 3.4 Implementation steps (concrete)

1. **Define `LogicalTime`** — e.g. `u64` or a small struct with `Ord` and `Default`. Put it in a new module, e.g. `streamweave::time` or `streamweave::progress`.
2. **Define `Timestamped<T>`** — struct with `payload: T` and `time: LogicalTime`; implement `Clone`, `Send`, `Sync` as needed.
3. **Timestamped input handle** — API for external drivers: `send(payload)`, `advance_to(time)`, optional `close()`. Internally, this feeds a channel or stream that the graph execution uses; each sent item is paired with the handle’s current time.
4. **Execution wrapper** — In `run_dataflow` (or a parallel path), wrap streams so that:
   - Incoming external data is already timestamped (from the handle).
   - Incoming internal data is either already `Timestamped` or is wrapped with a default/inherited time.
   - Outgoing data is forwarded as `Timestamped`; when it is consumed at a sink or at a progress point, update the completed frontier.
5. **Progress handle** — Expose `less_than(t)` / `less_equal(t)` or a progress stream, computed from the completed frontier.
6. **Documentation and tests** — Document the contract (monotonic advance_to, meaning of progress), and add tests that (1) run a simple pipeline with timestamps and (2) observe progress after advance_to.

### 3.5 What we explicitly defer

- **PathSummary / nested scopes** — Not needed for flat graphs; add when we introduce nested scopes or timestamp transformation along paths.
- **Distributed progress** — Single-worker progress is enough for the first version; multi-worker progress (antichains, capability exchange) comes with distribution.
- **Batch-by-timestamp operator API** — Operators can still process item-by-item; we only ensure every item has a time. A future `inspect_batch`-style API can group by timestamp when needed.

---

## 4. Summary

| Aspect | Timely | StreamWeave (today) | StreamWeave (proposed) |
|--------|--------|----------------------|-------------------------|
| Item type | `(data, timestamp)` | `Arc<dyn Any>` | `Timestamped<Payload>` (or wrapped) |
| Timestamp type | `T: Timestamp` (PartialOrder + Ord + minimum) | — | `LogicalTime` (Ord + Default) |
| Source | InputHandle + advance_to | Channels only | Timestamped input handle + advance_to |
| Progress | Probe (less_than / less_equal) | None | Progress handle or stream |
| Operators | Batch-by-timestamp (inspect_batch) | Item-by-item | Item-by-item first; batch API later |

Logical timestamps in Timely are **mandatory and central**: they provide correlation, ordering, and progress with minimal API (send, advance_to, probe). StreamWeave can adopt the same idea by (1) introducing `LogicalTime` and `Timestamped<T>`, (2) adding a timestamped input handle and advance_to, (3) wrapping existing streams with a default time and maintaining a completed frontier, and (4) exposing progress. Keeping the current `Node` API and using a wrapper preserves backward compatibility while making timestamps the base for future features (event-time, windowing, cycles, determinism).

---

## References

- [Timely Dataflow: Timestamps](https://timelydataflow.github.io/timely-dataflow/chapter_1/chapter_1_2.html)
- [Timely Dataflow: Tracking Progress](https://timelydataflow.github.io/timely-dataflow/chapter_1/chapter_1_3.html)
- [Timely Dataflow: Providing Input](https://timelydataflow.github.io/timely-dataflow/chapter_3/chapter_3_1.html)
- [timely::progress::timestamp::Timestamp](https://docs.rs/timely/latest/timely/progress/timestamp/trait.Timestamp.html)
- StreamWeave: [Gap Analysis and Implementation Strategy](streamweave-gap-analysis-and-implementation-strategy.md)
