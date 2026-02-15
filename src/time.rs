//! Logical timestamps for dataflow progress and ordering.
//!
//! This module provides [`LogicalTime`] and [`Timestamped`], the foundational
//! types for correlating inputs and outputs, ordering work, and tracking
//! progress in StreamWeave (see `docs/logical-timestamps-timely-and-streamweave.md`).
//!
//! Logical time is not wall-clock time; it can be a sequence number, batch id,
//! or round index. The default value is the minimum and is used as the initial
//! capability. [`Timestamped<T>`] attaches a logical time to a payload.
//!
//! ## Event time vs processing time
//!
//! StreamWeave distinguishes two time semantics:
//!
//! - **Event time**: The time when the event actually occurred (e.g. when a click happened,
//!   from payload or metadata). Use for correct ordering, windowing, and analytics when
//!   events arrive out of order. Implement [`HasEventTime`] for payload types that carry
//!   event time; event-time sources emit `Timestamped { payload, time: event_time }`.
//!
//! - **Processing time**: The time when the system processes the event (e.g. wall clock).
//!   Use for simple pipelines where arrival order is sufficient. Existing `TimestampNode`
//!   uses processing time. No late-data handling; everything is considered "on time."
//!
//! See [docs/event-time-semantics.md](../docs/event-time-semantics.md) for details.
//!
//! ## Progress contract
//!
//! - **Monotonic `advance_to`**: When using [`TimestampedInputHandle`], you must call
//!   [`advance_to`](TimestampedInputHandle::advance_to) with non-decreasing times.
//!   Calling with a smaller time panics. [`CompletedFrontier::advance_to`] is
//!   forward-only (smaller times are ignored).
//! - **Meaning of progress**: The completed frontier at a sink is the minimum logical
//!   time such that all items with time ≤ that value have been delivered to that
//!   sink. So `progress.less_than(t)` means "we may still see data with time < t";
//!   when it is false, we have completed at least up to time t.

use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::Mutex;

/// Logical time attached to dataflow items for ordering and progress.
///
/// Used to correlate inputs and outputs, define "minimum completed time"
/// (progress), and support rounds in iterative dataflows. Implements [`Ord`]
/// and [`Default`] (0) so it can be used as a totally ordered timestamp.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct LogicalTime(pub u64);

impl Default for LogicalTime {
    fn default() -> Self {
        Self(0)
    }
}

impl LogicalTime {
    /// Creates a new logical time from a raw value.
    #[inline]
    pub const fn new(t: u64) -> Self {
        Self(t)
    }

    /// Returns the raw u64 value.
    #[inline]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Returns the minimum logical time (same as `Default::default()`).
    #[inline]
    pub const fn minimum() -> Self {
        Self(0)
    }
}

// Send + Sync for use across threads (e.g. in streams and channels).
// LogicalTime is a newtype over u64, which is Send + Sync.
unsafe impl Send for LogicalTime {}
unsafe impl Sync for LogicalTime {}

/// Trait for payload types that carry event time.
///
/// Implement this for events that have a meaningful occurrence time (e.g. from Kafka
/// `timestamp`, a `created_at` field, or log timestamp). Event-time sources and window
/// nodes use this to extract the time for ordering and window assignment.
///
/// # Convention
///
/// - Return milliseconds since Unix epoch (or another monotonic epoch) as a `u64`.
/// - Return `None` if the event has no event time (e.g. synthetic or test data).
/// - For events with event time, prefer implementing this trait over ad-hoc extraction.
///
/// # Example
///
/// ```rust
/// use streamweave::time::{HasEventTime, LogicalTime};
///
/// struct ClickEvent {
///     user_id: u64,
///     created_at_ms: u64,
/// }
///
/// impl HasEventTime for ClickEvent {
///     fn event_time_ms(&self) -> Option<u64> {
///         Some(self.created_at_ms)
///     }
/// }
/// ```
pub trait HasEventTime {
    /// Returns the event time in milliseconds (e.g. since Unix epoch), or `None` if
    /// this event has no event time.
    fn event_time_ms(&self) -> Option<u64>;
}

impl HasEventTime for () {
    fn event_time_ms(&self) -> Option<u64> {
        None
    }
}

/// A payload with an attached logical timestamp.
///
/// Used as the envelope for all data in the timestamped execution path so that
/// the runtime can order work, track progress, and correlate inputs and outputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Timestamped<T> {
    /// The logical time of this item (batch, round, or sequence).
    pub time: LogicalTime,
    /// The payload.
    pub payload: T,
}

impl<T> Timestamped<T> {
    /// Creates a new timestamped item.
    #[inline]
    pub const fn new(payload: T, time: LogicalTime) -> Self {
        Self { time, payload }
    }

    /// Returns a reference to the payload.
    #[inline]
    pub const fn payload(&self) -> &T {
        &self.payload
    }

    /// Returns the logical time.
    #[inline]
    pub const fn time(&self) -> LogicalTime {
        self.time
    }
}

// Send/Sync when the payload is Send/Sync (for use in streams and channels).
unsafe impl<T: Send> Send for Timestamped<T> {}
unsafe impl<T: Sync> Sync for Timestamped<T> {}

/// Stream message: either data with timestamp or a watermark.
///
/// Channels that carry "data or watermark" use `StreamMessage<Payload>`. Sources (or a
/// watermark injector node) can emit `Watermark(T)` when they know no more data with
/// time < T will arrive. Nodes that buffer by time (e.g. event-time windows) consume
/// both: on `Watermark(T)` they advance their completed frontier and may flush windows.
/// Nodes that only care about data can strip watermarks.
///
/// See [docs/progress-tracking.md](../docs/progress-tracking.md).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamMessage<T> {
    /// Timestamped data item.
    Data(Timestamped<T>),
    /// Watermark: no more data with logical time < `t` will arrive.
    Watermark(LogicalTime),
}

unsafe impl<T: Send> Send for StreamMessage<T> {}
unsafe impl<T: Sync> Sync for StreamMessage<T> {}

impl<T> StreamMessage<T> {
    /// Returns `Some(&Timestamped<T>)` if this is `Data`, otherwise `None`.
    #[inline]
    pub fn data(&self) -> Option<&Timestamped<T>> {
        match self {
            Self::Data(ts) => Some(ts),
            Self::Watermark(_) => None,
        }
    }

    /// Returns `Some(LogicalTime)` if this is `Watermark`, otherwise `None`.
    #[inline]
    pub fn watermark(&self) -> Option<LogicalTime> {
        match self {
            Self::Data(_) => None,
            Self::Watermark(t) => Some(*t),
        }
    }

    /// Returns `true` if this is `Data`.
    #[inline]
    pub fn is_data(&self) -> bool {
        matches!(self, Self::Data(_))
    }

    /// Returns `true` if this is `Watermark`.
    #[inline]
    pub fn is_watermark(&self) -> bool {
        matches!(self, Self::Watermark(_))
    }
}

/// A payload with logical timestamp and multiplicity (diff) for differential dataflow.
///
/// In differential dataflow, each record carries a **multiplicity** (diff): **+1** =
/// insert, **−1** = retract. Operators compute and propagate only **changes**; downstream
/// can integrate diffs to obtain the current collection. This enables efficient
/// incremental and reactive updates.
///
/// See [docs/timestamped-differential-dataflow.md](../docs/timestamped-differential-dataflow.md).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DifferentialElement<T> {
    /// The payload.
    pub payload: T,
    /// Logical time for ordering and progress.
    pub time: LogicalTime,
    /// Multiplicity: +1 insert, −1 retract; +k/−k for batch updates.
    pub diff: i64,
}

impl<T> DifferentialElement<T> {
    /// Creates a new differential element.
    #[inline]
    pub const fn new(payload: T, time: LogicalTime, diff: i64) -> Self {
        Self { payload, time, diff }
    }

    /// Creates an insert (+1) at the given time.
    #[inline]
    pub fn insert(payload: T, time: LogicalTime) -> Self {
        Self::new(payload, time, 1)
    }

    /// Creates a retract (−1) at the given time.
    #[inline]
    pub fn retract(payload: T, time: LogicalTime) -> Self {
        Self::new(payload, time, -1)
    }

    /// Returns the payload.
    #[inline]
    pub const fn payload(&self) -> &T {
        &self.payload
    }

    /// Returns the logical time.
    #[inline]
    pub const fn time(&self) -> LogicalTime {
        self.time
    }

    /// Returns the diff (multiplicity).
    #[inline]
    pub const fn diff(&self) -> i64 {
        self.diff
    }

    /// Converts to [`Timestamped`] by dropping the diff (for compatibility with non-differential nodes).
    #[inline]
    pub fn into_timestamped(self) -> Timestamped<T> {
        Timestamped::new(self.payload, self.time)
    }
}

unsafe impl<T: Send> Send for DifferentialElement<T> {}
unsafe impl<T: Sync> Sync for DifferentialElement<T> {}

/// Stream message for the differential execution path: data with (payload, time, diff) or watermark.
///
/// Channels in the **differential execution path** carry `DifferentialStreamMessage<Payload>`.
/// Differential-aware nodes consume and produce this type. Watermarks propagate progress
/// as in the standard path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DifferentialStreamMessage<T> {
    /// Differential data element (payload, time, diff).
    Data(DifferentialElement<T>),
    /// Watermark: no more data with logical time &lt; `t` will arrive.
    Watermark(LogicalTime),
}

unsafe impl<T: Send> Send for DifferentialStreamMessage<T> {}
unsafe impl<T: Sync> Sync for DifferentialStreamMessage<T> {}

impl<T> DifferentialStreamMessage<T> {
    /// Returns `Some(&DifferentialElement<T>)` if this is `Data`, otherwise `None`.
    #[inline]
    pub fn data(&self) -> Option<&DifferentialElement<T>> {
        match self {
            Self::Data(elem) => Some(elem),
            Self::Watermark(_) => None,
        }
    }

    /// Returns `Some(LogicalTime)` if this is `Watermark`, otherwise `None`.
    #[inline]
    pub fn watermark(&self) -> Option<LogicalTime> {
        match self {
            Self::Data(_) => None,
            Self::Watermark(t) => Some(*t),
        }
    }
}

/// Shared state for the minimum logical time that has been completed (single-worker).
///
/// Updated by the execution layer when all items with time ≤ some value have
/// been consumed at the progress point. Read by the progress handle to expose
/// `less_than(t)` / `less_equal(t)`.
#[derive(Debug, Default)]
pub struct CompletedFrontier(AtomicU64);

impl CompletedFrontier {
    /// Creates a new completed frontier at the minimum time (0).
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current completed frontier (minimum time that has been completed).
    #[inline]
    pub fn get(&self) -> LogicalTime {
        LogicalTime(self.0.load(AtomicOrdering::Acquire))
    }

    /// Advances the frontier to at least `t`. Only moves forward; calling with
    /// a smaller time has no effect.
    #[inline]
    pub fn advance_to(&self, t: LogicalTime) {
        self.0.fetch_max(t.as_u64(), AtomicOrdering::Release);
    }
}

impl Clone for CompletedFrontier {
    fn clone(&self) -> Self {
        Self(AtomicU64::new(self.0.load(AtomicOrdering::Acquire)))
    }
}

// AtomicU64 is Send + Sync.
unsafe impl Send for CompletedFrontier {}
unsafe impl Sync for CompletedFrontier {}

/// Handle for feeding timestamped data into a dataflow (Timely-style input).
///
/// Hold a capability (current time); [`send`](TimestampedInputHandle::send) emits
/// `Timestamped { payload, time: current_time }`. [`advance_to`](TimestampedInputHandle::advance_to)
/// advances the capability monotonically. Drop or call [`close`](TimestampedInputHandle::close)
/// to signal no more data.
pub struct TimestampedInputHandle<T> {
    tx: tokio::sync::mpsc::Sender<Timestamped<T>>,
    current_time: Mutex<LogicalTime>,
}

impl<T: Send> TimestampedInputHandle<T> {
    /// Creates a new handle that sends to the given channel.
    /// The initial capability is [`LogicalTime::minimum()`].
    pub fn new(tx: tokio::sync::mpsc::Sender<Timestamped<T>>) -> Self {
        Self {
            tx,
            current_time: Mutex::new(LogicalTime::minimum()),
        }
    }

    /// Sends a payload with the current capability (time). Non-blocking; returns
    /// an error if the channel is closed or full.
    pub fn send(
        &self,
        payload: T,
    ) -> Result<(), tokio::sync::mpsc::error::TrySendError<Timestamped<T>>> {
        let time = *self.current_time.lock().expect("lock");
        self.tx.try_send(Timestamped::new(payload, time))
    }

    /// Advances the capability to at least `t`. Monotonic: panics if `t` is less
    /// than the current time.
    pub fn advance_to(&self, t: LogicalTime) {
        let mut cur = self.current_time.lock().expect("lock");
        if t < *cur {
            panic!(
                "advance_to({:?}) called but current time is {:?}; must be monotonic",
                t, *cur
            );
        }
        *cur = t;
    }

    /// Returns the current capability (time).
    pub fn time(&self) -> LogicalTime {
        *self.current_time.lock().expect("lock")
    }

    /// Consumes the handle and stops sending. Further sends are not possible.
    #[inline]
    pub fn close(self) {
        drop(self);
    }
}

// Sender<Timestamped<T>> is Send when T: Send; Mutex<LogicalTime> is Send.
unsafe impl<T: Send> Send for TimestampedInputHandle<T> {}
unsafe impl<T: Send> Sync for TimestampedInputHandle<T> {}

/// Probe-like handle to query progress (non-blocking).
///
/// Answers whether it is still possible to see data with timestamp less than
/// (or less or equal to) a given time at the associated point in the dataflow.
///
/// When created from multiple frontiers (e.g. per-sink), [`frontier`](Self::frontier)
/// returns the **minimum** over all frontiers (graph-level progress = "all sinks have
/// completed up to T").
#[derive(Clone, Debug)]
pub struct ProgressHandle {
    frontiers: Vec<std::sync::Arc<CompletedFrontier>>,
}

impl ProgressHandle {
    /// Creates a progress handle that reads from a single completed frontier.
    pub fn new(frontier: std::sync::Arc<CompletedFrontier>) -> Self {
        Self {
            frontiers: vec![frontier],
        }
    }

    /// Creates a progress handle from multiple frontiers (e.g. one per sink).
    /// Graph-level progress is the minimum over all frontiers: "all sinks have
    /// completed up to T" when `frontier() >= T`.
    pub fn from_frontiers(frontiers: Vec<std::sync::Arc<CompletedFrontier>>) -> Self {
        Self { frontiers }
    }

    fn effective_frontier(&self) -> LogicalTime {
        self.frontiers
            .iter()
            .map(|f| f.get())
            .min()
            .unwrap_or(LogicalTime::minimum())
    }

    /// Returns whether it is still possible to see a timestamp less than `t`.
    /// `true` means the frontier has not yet reached `t`; `false` means we have
    /// completed at least up to `t`.
    #[inline]
    pub fn less_than(&self, t: LogicalTime) -> bool {
        self.effective_frontier() < t
    }

    /// Returns whether it is still possible to see a timestamp less or equal to `t`.
    /// Once the frontier is greater than `t` (we have completed past `t`), this returns `false`.
    #[inline]
    pub fn less_equal(&self, t: LogicalTime) -> bool {
        self.effective_frontier() <= t
    }

    /// Returns the current completed frontier (minimum time completed).
    /// With multiple frontiers (per-sink), this is the minimum over all of them.
    #[inline]
    pub fn frontier(&self) -> LogicalTime {
        self.effective_frontier()
    }
}

unsafe impl Send for ProgressHandle {}
unsafe impl Sync for ProgressHandle {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;
    use std::sync::Arc;

    #[test]
    fn default_is_minimum() {
        assert_eq!(LogicalTime::default(), LogicalTime::minimum());
        assert_eq!(LogicalTime::default().as_u64(), 0);
    }

    #[test]
    fn ordering() {
        assert!(LogicalTime(0) < LogicalTime(1));
        assert!(LogicalTime(1) > LogicalTime(0));
        assert_eq!(LogicalTime(42).cmp(&LogicalTime(42)), Ordering::Equal);
    }

    #[test]
    fn send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LogicalTime>();
    }

    #[test]
    fn timestamped_basic() {
        let t = Timestamped::new(42u64, LogicalTime::new(1));
        assert_eq!(t.time(), LogicalTime::new(1));
        assert_eq!(t.payload(), &42);
        assert_eq!(t.payload, 42);
        assert_eq!(t.time, LogicalTime::new(1));
    }

    #[test]
    fn timestamped_clone() {
        let t = Timestamped::new(String::from("hi"), LogicalTime::default());
        let u = t.clone();
        assert_eq!(t.payload, u.payload);
        assert_eq!(t.time, u.time);
    }

    #[test]
    fn timestamped_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Timestamped<u64>>();
        assert_send_sync::<Timestamped<String>>();
    }

    #[test]
    fn differential_element_insert_retract() {
        let ins = DifferentialElement::insert(42i32, LogicalTime::new(1));
        assert_eq!(ins.payload(), &42);
        assert_eq!(ins.time(), LogicalTime::new(1));
        assert_eq!(ins.diff(), 1);

        let ret = DifferentialElement::retract(42i32, LogicalTime::new(1));
        assert_eq!(ret.diff(), -1);
    }

    #[test]
    fn differential_element_into_timestamped() {
        let elem = DifferentialElement::insert(99u64, LogicalTime::new(5));
        let ts = elem.into_timestamped();
        assert_eq!(ts.payload, 99);
        assert_eq!(ts.time, LogicalTime::new(5));
    }

    #[test]
    fn differential_stream_message_data_watermark() {
        let msg = DifferentialStreamMessage::Data(DifferentialElement::insert(
            String::from("x"),
            LogicalTime::new(2),
        ));
        assert!(msg.data().is_some());
        assert!(msg.watermark().is_none());

        let wm = DifferentialStreamMessage::<()>::Watermark(LogicalTime::new(3));
        assert!(wm.data().is_none());
        assert_eq!(wm.watermark(), Some(LogicalTime::new(3)));
    }

    #[test]
    fn completed_frontier_default() {
        let f = CompletedFrontier::new();
        assert_eq!(f.get(), LogicalTime::minimum());
    }

    #[test]
    fn completed_frontier_advance() {
        let f = CompletedFrontier::new();
        f.advance_to(LogicalTime::new(5));
        assert_eq!(f.get(), LogicalTime::new(5));
        f.advance_to(LogicalTime::new(3)); // no effect
        assert_eq!(f.get(), LogicalTime::new(5));
        f.advance_to(LogicalTime::new(10));
        assert_eq!(f.get(), LogicalTime::new(10));
    }

    #[test]
    fn completed_frontier_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<CompletedFrontier>();
    }

    #[test]
    fn timestamped_input_handle_send_advance() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Timestamped<u64>>(4);
        let handle = TimestampedInputHandle::<u64>::new(tx);
        assert_eq!(handle.time(), LogicalTime::minimum());
        handle.send(1u64).unwrap();
        handle.send(2u64).unwrap();
        handle.advance_to(LogicalTime::new(1));
        handle.send(3u64).unwrap();
        handle.advance_to(LogicalTime::new(2));
        handle.send(4u64).unwrap();
        drop(handle); // close

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            assert_eq!(rx.recv().await.unwrap().time, LogicalTime::new(0));
            assert_eq!(rx.recv().await.unwrap().time, LogicalTime::new(0));
            assert_eq!(rx.recv().await.unwrap().time, LogicalTime::new(1));
            assert_eq!(rx.recv().await.unwrap().time, LogicalTime::new(2));
            assert!(rx.recv().await.is_none());
        });
    }

    #[test]
    #[should_panic(expected = "must be monotonic")]
    fn timestamped_input_handle_advance_panic() {
        let (tx, _rx) = tokio::sync::mpsc::channel::<Timestamped<u64>>(1);
        let handle = TimestampedInputHandle::<u64>::new(tx);
        handle.advance_to(LogicalTime::new(2));
        handle.advance_to(LogicalTime::new(1)); // panic
    }

    #[test]
    fn progress_handle_less_than() {
        let frontier = std::sync::Arc::new(CompletedFrontier::new());
        let probe = ProgressHandle::new(frontier.clone());
        assert!(probe.less_than(LogicalTime::new(1)));
        assert!(probe.less_equal(LogicalTime::new(0)));
        frontier.advance_to(LogicalTime::new(1));
        assert!(!probe.less_than(LogicalTime::new(1)));
        assert!(probe.less_than(LogicalTime::new(2)));
        assert!(probe.less_equal(LogicalTime::new(1)));
        frontier.advance_to(LogicalTime::new(2));
        assert!(!probe.less_equal(LogicalTime::new(1)));
        assert_eq!(probe.frontier(), LogicalTime::new(2));
    }

    #[test]
    fn stream_message_data_and_watermark() {
        let ts = Timestamped::new(42u64, LogicalTime::new(1));
        let data = StreamMessage::Data(ts.clone());
        assert!(data.is_data());
        assert!(!data.is_watermark());
        assert_eq!(data.data(), Some(&ts));
        assert_eq!(data.watermark(), None);

        let wm = StreamMessage::<u64>::Watermark(LogicalTime::new(5));
        assert!(!wm.is_data());
        assert!(wm.is_watermark());
        assert_eq!(wm.data(), None);
        assert_eq!(wm.watermark(), Some(LogicalTime::new(5)));
    }

    #[test]
    fn progress_handle_from_frontiers_min() {
        let f1 = Arc::new(CompletedFrontier::new());
        let f2 = Arc::new(CompletedFrontier::new());
        let handle = ProgressHandle::from_frontiers(vec![f1.clone(), f2.clone()]);
        assert_eq!(handle.frontier(), LogicalTime::minimum());
        f1.advance_to(LogicalTime::new(5));
        assert_eq!(handle.frontier(), LogicalTime::minimum()); // f2 still at 0
        f2.advance_to(LogicalTime::new(3));
        assert_eq!(handle.frontier(), LogicalTime::new(3)); // min(5, 3) = 3
        f2.advance_to(LogicalTime::new(10));
        assert_eq!(handle.frontier(), LogicalTime::new(5)); // min(5, 10) = 5
    }

    /// Observe progress after advance_to: once frontier advances past t, less_equal(t) is false.
    #[test]
    fn observe_progress_after_advance_to() {
        let frontier = std::sync::Arc::new(CompletedFrontier::new());
        let progress = ProgressHandle::new(frontier.clone());
        assert!(progress.less_equal(LogicalTime::new(0)));
        frontier.advance_to(LogicalTime::new(1));
        assert!(progress.less_equal(LogicalTime::new(1)));
        frontier.advance_to(LogicalTime::new(2));
        assert!(!progress.less_equal(LogicalTime::new(1)));
        assert!(progress.less_equal(LogicalTime::new(2)));
    }
}
