//! Windowing operations for stream processing.
//!
//! This module provides abstractions for grouping stream elements into
//! bounded windows for aggregation and processing.
//!
//! # Overview
//!
//! Windowing is fundamental to stream processing, allowing bounded computations
//! over unbounded streams. Windows group elements based on:
//!
//! - **Time**: Group by event time or processing time
//! - **Count**: Group by number of elements
//! - **Session**: Group by activity with gaps
//!
//! # Core Concepts
//!
//! - [`TimeWindow`]: Represents a window boundary (start/end time)
//! - [`WindowAssigner`]: Assigns elements to windows
//! - [`WindowTrigger`]: Determines when to emit window results
//! - [`LateDataPolicy`]: Handles elements arriving after window closes
//!
//! # Window Types
//!
//! - [`TumblingWindowAssigner`]: Fixed-size, non-overlapping windows
//! - [`SlidingWindowAssigner`]: Fixed-size, overlapping windows
//! - [`SessionWindow`]: Gap-based dynamic windows
//!
//! # Example
//!
//! ```rust
//! use streamweave::window::{TimeWindow, TumblingWindowAssigner, WindowConfig};
//! use std::time::Duration;
//!
//! // Create a tumbling window of 5 seconds
//! let assigner = TumblingWindowAssigner::new(Duration::from_secs(5));
//! ```

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

/// Result of trigger evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerResult {
  /// Continue accumulating elements.
  Continue,
  /// Fire the window (emit results but keep state).
  Fire,
  /// Fire and purge (emit results and clear state).
  FireAndPurge,
  /// Purge without firing (discard state).
  Purge,
}

/// Policy for handling late data (elements arriving after window closes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LateDataPolicy {
  /// Drop late elements silently.
  #[default]
  Drop,
  /// Emit late elements to a side output.
  SideOutput,
  /// Include late elements in the window (refire if already fired).
  AllowLateness(Duration),
}

/// Error type for window operations.
#[derive(Debug)]
pub enum WindowError {
  /// Invalid window configuration.
  InvalidConfig(String),
  /// Window not found.
  NotFound(String),
  /// Window already closed.
  WindowClosed(String),
  /// State error.
  StateError(String),
}

impl fmt::Display for WindowError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      WindowError::InvalidConfig(msg) => write!(f, "Invalid window config: {}", msg),
      WindowError::NotFound(msg) => write!(f, "Window not found: {}", msg),
      WindowError::WindowClosed(msg) => write!(f, "Window closed: {}", msg),
      WindowError::StateError(msg) => write!(f, "State error: {}", msg),
    }
  }
}

impl std::error::Error for WindowError {}

/// Result type for window operations.
pub type WindowResult<T> = Result<T, WindowError>;

/// Trait for window types that can be used with `WindowState`.
///
/// This trait provides a common interface for different window types
/// (e.g., `TimeWindow`, `GlobalWindow`) to be used generically.
pub trait Window: Clone + Debug + Send + Sync + 'static {
  /// Returns the end time of the window, if applicable.
  /// Returns `None` for windows without a defined end (e.g., global windows).
  fn end_time(&self) -> Option<DateTime<Utc>>;

  /// Returns the start time of the window, if applicable.
  fn start_time(&self) -> Option<DateTime<Utc>>;

  /// Returns true if this window contains the given timestamp.
  fn contains(&self, timestamp: DateTime<Utc>) -> bool;
}

/// A time-based window with start and end timestamps.
///
/// This is the fundamental window type for time-based windowing strategies.
#[derive(Debug, Clone)]
pub struct TimeWindow {
  /// Start time of the window (inclusive).
  start: DateTime<Utc>,
  /// End time of the window (exclusive).
  end: DateTime<Utc>,
}

impl TimeWindow {
  /// Creates a new time window with the given start and end.
  pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
    Self { start, end }
  }

  /// Returns the start time of the window.
  pub fn start(&self) -> DateTime<Utc> {
    self.start
  }

  /// Returns the end time of the window.
  pub fn end(&self) -> DateTime<Utc> {
    self.end
  }

  /// Returns the duration of the window.
  pub fn duration(&self) -> ChronoDuration {
    self.end - self.start
  }

  /// Returns the maximum timestamp for elements in this window.
  pub fn max_timestamp(&self) -> DateTime<Utc> {
    self.end - ChronoDuration::milliseconds(1)
  }

  /// Returns true if the given timestamp falls within this window.
  pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
    timestamp >= self.start && timestamp < self.end
  }

  /// Returns true if this window intersects with another.
  pub fn intersects(&self, other: &TimeWindow) -> bool {
    self.start < other.end && other.start < self.end
  }

  /// Merges this window with another overlapping window.
  pub fn merge(&self, other: &TimeWindow) -> Option<TimeWindow> {
    if self.intersects(other) || self.end == other.start || other.end == self.start {
      Some(TimeWindow::new(
        self.start.min(other.start),
        self.end.max(other.end),
      ))
    } else {
      None
    }
  }
}

impl PartialEq for TimeWindow {
  fn eq(&self, other: &Self) -> bool {
    self.start == other.start && self.end == other.end
  }
}

impl Eq for TimeWindow {}

impl Hash for TimeWindow {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.start.hash(state);
    self.end.hash(state);
  }
}

impl PartialOrd for TimeWindow {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for TimeWindow {
  fn cmp(&self, other: &Self) -> Ordering {
    self
      .start
      .cmp(&other.start)
      .then_with(|| self.end.cmp(&other.end))
  }
}

impl fmt::Display for TimeWindow {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "[{}, {})",
      self.start.format("%H:%M:%S"),
      self.end.format("%H:%M:%S")
    )
  }
}

impl Window for TimeWindow {
  fn end_time(&self) -> Option<DateTime<Utc>> {
    Some(self.end)
  }

  fn start_time(&self) -> Option<DateTime<Utc>> {
    Some(self.start)
  }

  fn contains(&self, timestamp: DateTime<Utc>) -> bool {
    timestamp >= self.start && timestamp < self.end
  }
}

/// A count-based window that holds a fixed number of elements.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CountWindow {
  /// Window identifier (incremented for each new window).
  id: u64,
  /// Maximum count for this window.
  max_count: usize,
}

impl CountWindow {
  /// Creates a new count window with the given ID and max count.
  pub fn new(id: u64, max_count: usize) -> Self {
    Self { id, max_count }
  }

  /// Returns the window ID.
  pub fn id(&self) -> u64 {
    self.id
  }

  /// Returns the maximum count for this window.
  pub fn max_count(&self) -> usize {
    self.max_count
  }
}

impl fmt::Display for CountWindow {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "CountWindow(id={}, max={})", self.id, self.max_count)
  }
}

/// A session window with dynamic boundaries based on activity gaps.
#[derive(Debug, Clone)]
pub struct SessionWindow {
  /// Start time of the session.
  start: DateTime<Utc>,
  /// End time of the session.
  end: DateTime<Utc>,
  /// Session gap timeout.
  gap: Duration,
}

impl SessionWindow {
  /// Creates a new session window.
  pub fn new(start: DateTime<Utc>, end: DateTime<Utc>, gap: Duration) -> Self {
    Self { start, end, gap }
  }

  /// Creates a session window from a single element timestamp.
  pub fn from_element(timestamp: DateTime<Utc>, gap: Duration) -> Self {
    let gap_chrono = ChronoDuration::from_std(gap).unwrap_or(ChronoDuration::seconds(0));
    Self {
      start: timestamp,
      end: timestamp + gap_chrono,
      gap,
    }
  }

  /// Returns the start time.
  pub fn start(&self) -> DateTime<Utc> {
    self.start
  }

  /// Returns the end time.
  pub fn end(&self) -> DateTime<Utc> {
    self.end
  }

  /// Returns the gap duration.
  pub fn gap(&self) -> Duration {
    self.gap
  }

  /// Returns true if this session contains the timestamp.
  pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
    timestamp >= self.start && timestamp < self.end
  }

  /// Returns true if this session should merge with another.
  pub fn should_merge(&self, other: &SessionWindow) -> bool {
    // Sessions merge if they overlap or are adjacent (within gap)
    self.start < other.end && other.start < self.end
  }

  /// Merges this session with another.
  pub fn merge(&self, other: &SessionWindow) -> Option<SessionWindow> {
    if self.should_merge(other) {
      Some(SessionWindow::new(
        self.start.min(other.start),
        self.end.max(other.end),
        self.gap.max(other.gap),
      ))
    } else {
      None
    }
  }

  /// Extends the session to include a new timestamp.
  pub fn extend(&mut self, timestamp: DateTime<Utc>) {
    let gap_chrono = ChronoDuration::from_std(self.gap).unwrap_or(ChronoDuration::seconds(0));
    if timestamp < self.start {
      self.start = timestamp;
    }
    let new_end = timestamp + gap_chrono;
    if new_end > self.end {
      self.end = new_end;
    }
  }
}

impl PartialEq for SessionWindow {
  fn eq(&self, other: &Self) -> bool {
    self.start == other.start && self.end == other.end
  }
}

impl Eq for SessionWindow {}

impl Hash for SessionWindow {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.start.hash(state);
    self.end.hash(state);
  }
}

impl fmt::Display for SessionWindow {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Session[{}, {}) gap={:?}",
      self.start.format("%H:%M:%S"),
      self.end.format("%H:%M:%S"),
      self.gap
    )
  }
}

/// Trait for window assigners that assign elements to windows.
///
/// Window assigners determine which windows an element belongs to based on
/// its timestamp or other properties.
pub trait WindowAssigner: Send + Sync {
  /// The window type produced by this assigner.
  type W: Clone + Debug + PartialEq + Eq + Hash + Send + Sync;

  /// Assign an element to zero or more windows.
  ///
  /// # Arguments
  ///
  /// * `timestamp` - The timestamp of the element
  ///
  /// # Returns
  ///
  /// A vector of windows the element belongs to.
  fn assign_windows(&self, timestamp: DateTime<Utc>) -> Vec<Self::W>;

  /// Returns the default trigger for this assigner.
  fn default_trigger(&self) -> Box<dyn WindowTrigger<Self::W>>;

  /// Returns true if this assigner produces event-time windows.
  fn is_event_time(&self) -> bool {
    true
  }
}

/// Trait for window triggers that determine when to emit results.
///
/// Triggers are fired based on:
/// - Element arrival
/// - Processing time advance
/// - Event time (watermark) advance
pub trait WindowTrigger<W>: Send + Sync
where
  W: Clone + Debug + PartialEq + Eq + Hash,
{
  /// Called when an element is added to a window.
  fn on_element(&mut self, timestamp: DateTime<Utc>, window: &W) -> TriggerResult;

  /// Called when processing time advances.
  fn on_processing_time(&mut self, time: DateTime<Utc>, window: &W) -> TriggerResult;

  /// Called when the watermark (event time) advances.
  fn on_event_time(&mut self, watermark: DateTime<Utc>, window: &W) -> TriggerResult;

  /// Called when the trigger is cleared.
  fn clear(&mut self, window: &W);

  /// Creates a clone of this trigger.
  fn clone_trigger(&self) -> Box<dyn WindowTrigger<W>>;
}

/// Default trigger that fires when the watermark passes the window end.
#[derive(Debug, Clone, Default)]
pub struct EventTimeTrigger;

impl EventTimeTrigger {
  /// Creates a new event time trigger.
  pub fn new() -> Self {
    Self
  }
}

impl WindowTrigger<TimeWindow> for EventTimeTrigger {
  fn on_element(&mut self, _timestamp: DateTime<Utc>, _window: &TimeWindow) -> TriggerResult {
    TriggerResult::Continue
  }

  fn on_processing_time(&mut self, _time: DateTime<Utc>, _window: &TimeWindow) -> TriggerResult {
    TriggerResult::Continue
  }

  fn on_event_time(&mut self, watermark: DateTime<Utc>, window: &TimeWindow) -> TriggerResult {
    if watermark >= window.end() {
      TriggerResult::FireAndPurge
    } else {
      TriggerResult::Continue
    }
  }

  fn clear(&mut self, _window: &TimeWindow) {}

  fn clone_trigger(&self) -> Box<dyn WindowTrigger<TimeWindow>> {
    Box::new(self.clone())
  }
}

/// Trigger that fires after a specified count of elements.
#[derive(Debug, Clone)]
pub struct CountTrigger {
  /// Count at which to fire.
  count: usize,
  /// Current counts per window.
  window_counts: HashMap<CountWindow, usize>,
}

impl CountTrigger {
  /// Creates a new count trigger that fires after `count` elements.
  pub fn new(count: usize) -> Self {
    Self {
      count,
      window_counts: HashMap::new(),
    }
  }

  /// Returns the target count.
  pub fn count(&self) -> usize {
    self.count
  }
}

impl WindowTrigger<CountWindow> for CountTrigger {
  fn on_element(&mut self, _timestamp: DateTime<Utc>, window: &CountWindow) -> TriggerResult {
    let count = self.window_counts.entry(window.clone()).or_insert(0);
    *count += 1;

    if *count >= self.count {
      TriggerResult::FireAndPurge
    } else {
      TriggerResult::Continue
    }
  }

  fn on_processing_time(&mut self, _time: DateTime<Utc>, _window: &CountWindow) -> TriggerResult {
    TriggerResult::Continue
  }

  fn on_event_time(&mut self, _watermark: DateTime<Utc>, _window: &CountWindow) -> TriggerResult {
    TriggerResult::Continue
  }

  fn clear(&mut self, window: &CountWindow) {
    self.window_counts.remove(window);
  }

  fn clone_trigger(&self) -> Box<dyn WindowTrigger<CountWindow>> {
    Box::new(self.clone())
  }
}

/// Trigger that fires when the session gap timeout expires.
#[derive(Debug, Clone)]
pub struct SessionTrigger {
  /// Last event time per window.
  last_event_times: HashMap<SessionWindow, DateTime<Utc>>,
}

impl SessionTrigger {
  /// Creates a new session trigger.
  pub fn new() -> Self {
    Self {
      last_event_times: HashMap::new(),
    }
  }
}

impl Default for SessionTrigger {
  fn default() -> Self {
    Self::new()
  }
}

impl WindowTrigger<SessionWindow> for SessionTrigger {
  fn on_element(&mut self, timestamp: DateTime<Utc>, window: &SessionWindow) -> TriggerResult {
    self.last_event_times.insert(window.clone(), timestamp);
    TriggerResult::Continue
  }

  fn on_processing_time(&mut self, _time: DateTime<Utc>, _window: &SessionWindow) -> TriggerResult {
    TriggerResult::Continue
  }

  fn on_event_time(&mut self, watermark: DateTime<Utc>, window: &SessionWindow) -> TriggerResult {
    // Fire when watermark passes the session end
    if watermark >= window.end() {
      TriggerResult::FireAndPurge
    } else {
      TriggerResult::Continue
    }
  }

  fn clear(&mut self, window: &SessionWindow) {
    self.last_event_times.remove(window);
  }

  fn clone_trigger(&self) -> Box<dyn WindowTrigger<SessionWindow>> {
    Box::new(self.clone())
  }
}

/// Processing time trigger that fires at regular intervals.
#[derive(Debug, Clone)]
pub struct ProcessingTimeTrigger {
  /// Interval at which to fire.
  interval: Duration,
  /// Last fire time per window.
  last_fire_times: HashMap<TimeWindow, DateTime<Utc>>,
}

impl ProcessingTimeTrigger {
  /// Creates a new processing time trigger.
  pub fn new(interval: Duration) -> Self {
    Self {
      interval,
      last_fire_times: HashMap::new(),
    }
  }

  /// Returns the trigger interval.
  pub fn interval(&self) -> Duration {
    self.interval
  }
}

impl WindowTrigger<TimeWindow> for ProcessingTimeTrigger {
  fn on_element(&mut self, _timestamp: DateTime<Utc>, _window: &TimeWindow) -> TriggerResult {
    TriggerResult::Continue
  }

  fn on_processing_time(&mut self, time: DateTime<Utc>, window: &TimeWindow) -> TriggerResult {
    let interval_chrono =
      ChronoDuration::from_std(self.interval).unwrap_or(ChronoDuration::seconds(1));

    let last_fire = self
      .last_fire_times
      .get(window)
      .copied()
      .unwrap_or_else(|| time - interval_chrono);

    if time >= last_fire + interval_chrono {
      self.last_fire_times.insert(window.clone(), time);
      TriggerResult::Fire
    } else {
      TriggerResult::Continue
    }
  }

  fn on_event_time(&mut self, _watermark: DateTime<Utc>, _window: &TimeWindow) -> TriggerResult {
    TriggerResult::Continue
  }

  fn clear(&mut self, window: &TimeWindow) {
    self.last_fire_times.remove(window);
  }

  fn clone_trigger(&self) -> Box<dyn WindowTrigger<TimeWindow>> {
    Box::new(self.clone())
  }
}

/// Tumbling window assigner that creates non-overlapping windows.
///
/// Each element is assigned to exactly one window.
#[derive(Debug, Clone)]
pub struct TumblingWindowAssigner {
  /// Size of each window.
  size: Duration,
  /// Offset from epoch (for alignment).
  offset: Duration,
}

impl TumblingWindowAssigner {
  /// Creates a new tumbling window assigner with the given size.
  pub fn new(size: Duration) -> Self {
    Self {
      size,
      offset: Duration::ZERO,
    }
  }

  /// Sets the offset for window alignment.
  pub fn with_offset(mut self, offset: Duration) -> Self {
    self.offset = offset;
    self
  }

  /// Returns the window size.
  pub fn size(&self) -> Duration {
    self.size
  }

  /// Returns the offset.
  pub fn offset(&self) -> Duration {
    self.offset
  }

  /// Calculates the window start for a given timestamp.
  fn window_start(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
    let ts_millis = timestamp.timestamp_millis();
    let size_millis = self.size.as_millis() as i64;
    let offset_millis = self.offset.as_millis() as i64;

    let window_start_millis =
      ((ts_millis - offset_millis) / size_millis) * size_millis + offset_millis;

    DateTime::from_timestamp_millis(window_start_millis).unwrap_or(timestamp)
  }
}

impl WindowAssigner for TumblingWindowAssigner {
  type W = TimeWindow;

  fn assign_windows(&self, timestamp: DateTime<Utc>) -> Vec<Self::W> {
    let start = self.window_start(timestamp);
    let size_chrono = ChronoDuration::from_std(self.size).unwrap_or(ChronoDuration::seconds(1));
    let end = start + size_chrono;

    vec![TimeWindow::new(start, end)]
  }

  fn default_trigger(&self) -> Box<dyn WindowTrigger<Self::W>> {
    Box::new(EventTimeTrigger::new())
  }
}

/// Sliding window assigner that creates overlapping windows.
///
/// Each element may be assigned to multiple windows.
#[derive(Debug, Clone)]
pub struct SlidingWindowAssigner {
  /// Size of each window.
  size: Duration,
  /// Slide interval between windows.
  slide: Duration,
  /// Offset from epoch (for alignment).
  offset: Duration,
}

impl SlidingWindowAssigner {
  /// Creates a new sliding window assigner.
  pub fn new(size: Duration, slide: Duration) -> Self {
    Self {
      size,
      slide,
      offset: Duration::ZERO,
    }
  }

  /// Sets the offset for window alignment.
  pub fn with_offset(mut self, offset: Duration) -> Self {
    self.offset = offset;
    self
  }

  /// Returns the window size.
  pub fn size(&self) -> Duration {
    self.size
  }

  /// Returns the slide interval.
  pub fn slide(&self) -> Duration {
    self.slide
  }

  /// Returns the offset.
  pub fn offset(&self) -> Duration {
    self.offset
  }
}

impl WindowAssigner for SlidingWindowAssigner {
  type W = TimeWindow;

  fn assign_windows(&self, timestamp: DateTime<Utc>) -> Vec<Self::W> {
    let ts_millis = timestamp.timestamp_millis();
    let size_millis = self.size.as_millis() as i64;
    let slide_millis = self.slide.as_millis() as i64;
    let offset_millis = self.offset.as_millis() as i64;

    let size_chrono = ChronoDuration::from_std(self.size).unwrap_or(ChronoDuration::seconds(1));

    // Find the last window that could contain this timestamp
    let last_start = ((ts_millis - offset_millis) / slide_millis) * slide_millis + offset_millis;

    // Number of windows this element belongs to
    let num_windows = (size_millis / slide_millis) as usize;

    (0..num_windows)
      .map(|i| {
        let start_millis = last_start - (i as i64) * slide_millis;
        let start = DateTime::from_timestamp_millis(start_millis).unwrap_or(timestamp);
        let end = start + size_chrono;
        TimeWindow::new(start, end)
      })
      .filter(|w| w.contains(timestamp))
      .collect()
  }

  fn default_trigger(&self) -> Box<dyn WindowTrigger<Self::W>> {
    Box::new(EventTimeTrigger::new())
  }
}

/// Session window assigner that creates gap-based windows.
///
/// Elements are grouped into sessions based on activity gaps.
#[derive(Debug, Clone)]
pub struct SessionWindowAssigner {
  /// Gap duration that triggers a new session.
  gap: Duration,
}

impl SessionWindowAssigner {
  /// Creates a new session window assigner with the given gap.
  pub fn new(gap: Duration) -> Self {
    Self { gap }
  }

  /// Returns the gap duration.
  pub fn gap(&self) -> Duration {
    self.gap
  }
}

impl WindowAssigner for SessionWindowAssigner {
  type W = SessionWindow;

  fn assign_windows(&self, timestamp: DateTime<Utc>) -> Vec<Self::W> {
    vec![SessionWindow::from_element(timestamp, self.gap)]
  }

  fn default_trigger(&self) -> Box<dyn WindowTrigger<Self::W>> {
    Box::new(SessionTrigger::new())
  }
}

/// Count-based tumbling window assigner.
///
/// Creates windows based on element count rather than time.
#[derive(Debug)]
pub struct CountWindowAssigner {
  /// Number of elements per window.
  count: usize,
  /// Current window ID.
  current_id: std::sync::atomic::AtomicU64,
  /// Current count in the window.
  current_count: std::sync::atomic::AtomicUsize,
}

impl Clone for CountWindowAssigner {
  fn clone(&self) -> Self {
    Self {
      count: self.count,
      current_id: std::sync::atomic::AtomicU64::new(
        self.current_id.load(std::sync::atomic::Ordering::Relaxed),
      ),
      current_count: std::sync::atomic::AtomicUsize::new(
        self
          .current_count
          .load(std::sync::atomic::Ordering::Relaxed),
      ),
    }
  }
}

impl CountWindowAssigner {
  /// Creates a new count window assigner.
  pub fn new(count: usize) -> Self {
    Self {
      count,
      current_id: std::sync::atomic::AtomicU64::new(0),
      current_count: std::sync::atomic::AtomicUsize::new(0),
    }
  }

  /// Returns the count per window.
  pub fn count(&self) -> usize {
    self.count
  }
}

impl WindowAssigner for CountWindowAssigner {
  type W = CountWindow;

  fn assign_windows(&self, _timestamp: DateTime<Utc>) -> Vec<Self::W> {
    // Increment count
    let prev_count = self
      .current_count
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Check if we need a new window
    if prev_count > 0 && prev_count % self.count == 0 {
      self
        .current_id
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
      self
        .current_count
        .store(1, std::sync::atomic::Ordering::Relaxed);
    }

    let id = self.current_id.load(std::sync::atomic::Ordering::Relaxed);

    vec![CountWindow::new(id, self.count)]
  }

  fn default_trigger(&self) -> Box<dyn WindowTrigger<Self::W>> {
    Box::new(CountTrigger::new(self.count))
  }

  fn is_event_time(&self) -> bool {
    false
  }
}

/// Global window assigner that assigns all elements to a single window.
#[derive(Debug, Clone, Default)]
pub struct GlobalWindowAssigner;

/// Global window that contains all elements.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlobalWindow;

impl fmt::Display for GlobalWindow {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "GlobalWindow")
  }
}

impl Window for GlobalWindow {
  fn end_time(&self) -> Option<DateTime<Utc>> {
    None // Global window has no end
  }

  fn start_time(&self) -> Option<DateTime<Utc>> {
    None // Global window has no start
  }

  fn contains(&self, _timestamp: DateTime<Utc>) -> bool {
    true // Global window contains all timestamps
  }
}

impl GlobalWindowAssigner {
  /// Creates a new global window assigner.
  pub fn new() -> Self {
    Self
  }
}

impl WindowAssigner for GlobalWindowAssigner {
  type W = GlobalWindow;

  fn assign_windows(&self, _timestamp: DateTime<Utc>) -> Vec<Self::W> {
    vec![GlobalWindow]
  }

  fn default_trigger(&self) -> Box<dyn WindowTrigger<Self::W>> {
    Box::new(NeverTrigger)
  }
}

/// Trigger that never fires (for global windows).
#[derive(Debug, Clone, Default)]
pub struct NeverTrigger;

impl WindowTrigger<GlobalWindow> for NeverTrigger {
  fn on_element(&mut self, _timestamp: DateTime<Utc>, _window: &GlobalWindow) -> TriggerResult {
    TriggerResult::Continue
  }

  fn on_processing_time(&mut self, _time: DateTime<Utc>, _window: &GlobalWindow) -> TriggerResult {
    TriggerResult::Continue
  }

  fn on_event_time(&mut self, _watermark: DateTime<Utc>, _window: &GlobalWindow) -> TriggerResult {
    TriggerResult::Continue
  }

  fn clear(&mut self, _window: &GlobalWindow) {}

  fn clone_trigger(&self) -> Box<dyn WindowTrigger<GlobalWindow>> {
    Box::new(self.clone())
  }
}

/// Configuration for window operations.
#[derive(Debug, Clone)]
pub struct WindowConfig {
  /// Policy for handling late data.
  pub late_data_policy: LateDataPolicy,
  /// Allowed lateness for elements.
  pub allowed_lateness: Duration,
}

impl Default for WindowConfig {
  fn default() -> Self {
    Self {
      late_data_policy: LateDataPolicy::Drop,
      allowed_lateness: Duration::ZERO,
    }
  }
}

impl WindowConfig {
  /// Creates a new window configuration.
  pub fn new() -> Self {
    Self::default()
  }

  /// Sets the late data policy.
  pub fn with_late_data_policy(mut self, policy: LateDataPolicy) -> Self {
    self.late_data_policy = policy;
    self
  }

  /// Sets the allowed lateness.
  pub fn with_allowed_lateness(mut self, lateness: Duration) -> Self {
    self.allowed_lateness = lateness;
    self
  }
}

// =============================================================================
// Watermark Support
// =============================================================================

/// Represents a watermark in event-time processing.
///
/// A watermark is a statement that all events with timestamps less than or equal
/// to the watermark have arrived. It's used to track event-time progress and
/// determine when windows can be triggered.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Watermark {
  /// The timestamp represented by this watermark.
  pub timestamp: DateTime<Utc>,
}

impl Watermark {
  /// Creates a new watermark at the given timestamp.
  #[must_use]
  pub fn new(timestamp: DateTime<Utc>) -> Self {
    Self { timestamp }
  }

  /// Creates an initial watermark at the minimum possible time.
  #[must_use]
  pub fn min() -> Self {
    Self {
      timestamp: DateTime::<Utc>::MIN_UTC,
    }
  }

  /// Creates a watermark at the maximum possible time (end of stream).
  #[must_use]
  pub fn max() -> Self {
    Self {
      timestamp: DateTime::<Utc>::MAX_UTC,
    }
  }

  /// Returns true if this is the end-of-stream watermark.
  #[must_use]
  pub fn is_end_of_stream(&self) -> bool {
    self.timestamp == DateTime::<Utc>::MAX_UTC
  }

  /// Advances this watermark to at least the given timestamp.
  /// Returns true if the watermark was actually advanced.
  pub fn advance(&mut self, timestamp: DateTime<Utc>) -> bool {
    if timestamp > self.timestamp {
      self.timestamp = timestamp;
      true
    } else {
      false
    }
  }
}

impl Display for Watermark {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    if self.is_end_of_stream() {
      write!(f, "Watermark(END)")
    } else {
      write!(f, "Watermark({})", self.timestamp)
    }
  }
}

/// Trait for generating watermarks from observed event timestamps.
///
/// Different strategies exist for watermark generation:
/// - Monotonic: watermark follows the maximum observed timestamp exactly
/// - Bounded out-of-orderness: allows for some delay in event arrival
/// - Periodic: generates watermarks at fixed intervals
pub trait WatermarkGenerator: Send + Sync + std::fmt::Debug + 'static {
  /// Called when a new element arrives with the given event timestamp.
  /// Returns the new watermark if it should be emitted.
  fn on_event(&mut self, timestamp: DateTime<Utc>) -> Option<Watermark>;

  /// Called periodically to generate watermarks even without new events.
  /// Returns the new watermark if it should be emitted.
  fn on_periodic_emit(&mut self) -> Option<Watermark>;

  /// Returns the current watermark without advancing it.
  fn current_watermark(&self) -> Watermark;

  /// Signals end of stream, generating the final watermark.
  fn on_end_of_stream(&mut self) -> Watermark {
    Watermark::max()
  }
}

/// Generates watermarks that follow the maximum observed timestamp exactly.
///
/// This is suitable for streams where events arrive in order.
#[derive(Debug, Clone)]
pub struct MonotonicWatermarkGenerator {
  current: Watermark,
}

impl MonotonicWatermarkGenerator {
  /// Creates a new monotonic watermark generator.
  #[must_use]
  pub fn new() -> Self {
    Self {
      current: Watermark::min(),
    }
  }
}

impl Default for MonotonicWatermarkGenerator {
  fn default() -> Self {
    Self::new()
  }
}

impl WatermarkGenerator for MonotonicWatermarkGenerator {
  fn on_event(&mut self, timestamp: DateTime<Utc>) -> Option<Watermark> {
    if self.current.advance(timestamp) {
      Some(self.current.clone())
    } else {
      None
    }
  }

  fn on_periodic_emit(&mut self) -> Option<Watermark> {
    // Monotonic generator doesn't emit on periodic basis
    None
  }

  fn current_watermark(&self) -> Watermark {
    self.current.clone()
  }
}

/// Generates watermarks allowing for bounded out-of-orderness.
///
/// The watermark is set to `max_observed_timestamp - max_out_of_orderness`.
/// This allows events to arrive slightly out of order while still making progress.
#[derive(Debug, Clone)]
pub struct BoundedOutOfOrdernessGenerator {
  /// Maximum allowed out-of-orderness.
  max_out_of_orderness: Duration,
  /// Maximum timestamp seen so far.
  max_timestamp: Option<DateTime<Utc>>,
  /// Current watermark.
  current: Watermark,
}

impl BoundedOutOfOrdernessGenerator {
  /// Creates a new bounded out-of-orderness watermark generator.
  #[must_use]
  pub fn new(max_out_of_orderness: Duration) -> Self {
    Self {
      max_out_of_orderness,
      max_timestamp: None,
      current: Watermark::min(),
    }
  }

  fn calculate_watermark(&self) -> Watermark {
    match self.max_timestamp {
      Some(max_ts) => Watermark::new(max_ts - self.max_out_of_orderness),
      None => Watermark::min(),
    }
  }
}

impl WatermarkGenerator for BoundedOutOfOrdernessGenerator {
  fn on_event(&mut self, timestamp: DateTime<Utc>) -> Option<Watermark> {
    // Update max timestamp
    self.max_timestamp = Some(
      self
        .max_timestamp
        .map(|current| current.max(timestamp))
        .unwrap_or(timestamp),
    );

    // Calculate new watermark
    let new_watermark = self.calculate_watermark();

    // Only emit if watermark advanced
    if new_watermark.timestamp > self.current.timestamp {
      self.current = new_watermark.clone();
      Some(new_watermark)
    } else {
      None
    }
  }

  fn on_periodic_emit(&mut self) -> Option<Watermark> {
    // Emit current watermark on periodic basis
    Some(self.current.clone())
  }

  fn current_watermark(&self) -> Watermark {
    self.current.clone()
  }
}

/// Generates watermarks at fixed intervals regardless of event arrival.
#[derive(Debug, Clone)]
pub struct PeriodicWatermarkGenerator {
  /// The inner generator for calculating watermark values.
  inner: BoundedOutOfOrdernessGenerator,
  /// Interval between watermark emissions.
  interval: Duration,
  /// Last emission time.
  last_emit: Option<DateTime<Utc>>,
}

impl PeriodicWatermarkGenerator {
  /// Creates a new periodic watermark generator.
  #[must_use]
  pub fn new(max_out_of_orderness: Duration, interval: Duration) -> Self {
    Self {
      inner: BoundedOutOfOrdernessGenerator::new(max_out_of_orderness),
      interval,
      last_emit: None,
    }
  }
}

impl WatermarkGenerator for PeriodicWatermarkGenerator {
  fn on_event(&mut self, timestamp: DateTime<Utc>) -> Option<Watermark> {
    // Update the inner generator but don't emit watermark per-event
    self.inner.on_event(timestamp);
    None
  }

  fn on_periodic_emit(&mut self) -> Option<Watermark> {
    let now = Utc::now();
    let interval_chrono = ChronoDuration::from_std(self.interval).unwrap_or(ChronoDuration::zero());
    let should_emit = self
      .last_emit
      .map(|last| now - last >= interval_chrono)
      .unwrap_or(true);

    if should_emit {
      self.last_emit = Some(now);
      Some(self.inner.current_watermark())
    } else {
      None
    }
  }

  fn current_watermark(&self) -> Watermark {
    self.inner.current_watermark()
  }
}

// =============================================================================
// Late Data Handling
// =============================================================================

/// Result of evaluating whether an element is late.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LateDataResult<T> {
  /// The element is not late and should be processed normally.
  OnTime(T),
  /// The element is late but within allowed lateness.
  WithinLateness(T),
  /// The element is late and should be dropped.
  Drop,
  /// The element is late and should be redirected to side output.
  SideOutput(T),
}

impl<T> LateDataResult<T> {
  /// Returns true if the element should be processed (OnTime or WithinLateness).
  #[must_use]
  pub fn should_process(&self) -> bool {
    matches!(
      self,
      LateDataResult::OnTime(_) | LateDataResult::WithinLateness(_)
    )
  }

  /// Extracts the element if it should be processed.
  #[must_use]
  pub fn into_processable(self) -> Option<T> {
    match self {
      LateDataResult::OnTime(t) | LateDataResult::WithinLateness(t) => Some(t),
      _ => None,
    }
  }

  /// Returns true if this is a side output.
  #[must_use]
  pub fn is_side_output(&self) -> bool {
    matches!(self, LateDataResult::SideOutput(_))
  }

  /// Extracts the element for side output.
  #[must_use]
  pub fn into_side_output(self) -> Option<T> {
    match self {
      LateDataResult::SideOutput(t) => Some(t),
      _ => None,
    }
  }
}

/// Handles late data according to the configured policy.
#[derive(Debug, Clone)]
pub struct LateDataHandler {
  /// Policy for handling late data.
  policy: LateDataPolicy,
  /// Statistics about late data.
  stats: LateDataStats,
}

/// Statistics about late data processing.
#[derive(Debug, Clone, Default)]
pub struct LateDataStats {
  /// Number of on-time elements.
  pub on_time: u64,
  /// Number of elements within allowed lateness.
  pub within_lateness: u64,
  /// Number of dropped late elements.
  pub dropped: u64,
  /// Number of elements sent to side output.
  pub side_output: u64,
}

impl LateDataHandler {
  /// Creates a new late data handler with the given policy.
  #[must_use]
  pub fn new(policy: LateDataPolicy) -> Self {
    Self {
      policy,
      stats: LateDataStats::default(),
    }
  }

  /// Creates a handler that drops all late data.
  #[must_use]
  pub fn drop_late() -> Self {
    Self::new(LateDataPolicy::Drop)
  }

  /// Creates a handler that allows late data within the given lateness window.
  #[must_use]
  pub fn with_allowed_lateness(lateness: Duration) -> Self {
    Self::new(LateDataPolicy::AllowLateness(lateness))
  }

  /// Creates a handler that redirects late data to a side output.
  #[must_use]
  pub fn redirect_to_side_output() -> Self {
    Self::new(LateDataPolicy::SideOutput)
  }

  /// Returns the allowed lateness based on the policy.
  fn allowed_lateness(&self) -> Duration {
    match &self.policy {
      LateDataPolicy::AllowLateness(d) => *d,
      _ => Duration::ZERO,
    }
  }

  /// Evaluates whether an element is late and returns the appropriate action.
  ///
  /// # Arguments
  /// - `element_timestamp`: The event time of the element.
  /// - `current_watermark`: The current watermark.
  /// - `element`: The element to evaluate.
  pub fn evaluate<T>(
    &mut self,
    element_timestamp: DateTime<Utc>,
    current_watermark: &Watermark,
    element: T,
  ) -> LateDataResult<T> {
    // Element is on-time if its timestamp >= watermark
    if element_timestamp >= current_watermark.timestamp {
      self.stats.on_time += 1;
      return LateDataResult::OnTime(element);
    }

    // Element is late - check if within allowed lateness
    let lateness_duration = current_watermark.timestamp - element_timestamp;
    let allowed = self.allowed_lateness();

    // Convert chrono Duration to std Duration for comparison
    if let Ok(lateness_std) = lateness_duration.to_std()
      && lateness_std <= allowed
    {
      self.stats.within_lateness += 1;
      return LateDataResult::WithinLateness(element);
    }

    // Element is truly late - apply policy
    match &self.policy {
      LateDataPolicy::Drop => {
        self.stats.dropped += 1;
        LateDataResult::Drop
      }
      LateDataPolicy::AllowLateness(_) => {
        // Even beyond allowed lateness, process it (policy says to allow)
        self.stats.within_lateness += 1;
        LateDataResult::WithinLateness(element)
      }
      LateDataPolicy::SideOutput => {
        self.stats.side_output += 1;
        LateDataResult::SideOutput(element)
      }
    }
  }

  /// Returns statistics about late data handling.
  #[must_use]
  pub fn stats(&self) -> &LateDataStats {
    &self.stats
  }

  /// Resets statistics.
  pub fn reset_stats(&mut self) {
    self.stats = LateDataStats::default();
  }

  /// Returns the configured policy.
  #[must_use]
  pub fn policy(&self) -> &LateDataPolicy {
    &self.policy
  }

  /// Returns the allowed lateness from the policy.
  #[must_use]
  pub fn get_allowed_lateness(&self) -> Duration {
    match &self.policy {
      LateDataPolicy::AllowLateness(d) => *d,
      _ => Duration::ZERO,
    }
  }
}

// =============================================================================
// Windowed Stream Processing
// =============================================================================

/// Tracks the state of a window including its watermark context.
#[derive(Debug)]
pub struct WindowState<W: Window, T> {
  /// The window this state belongs to.
  pub window: W,
  /// Elements currently in the window.
  pub elements: Vec<(DateTime<Utc>, T)>,
  /// Count of elements.
  pub count: usize,
  /// Whether the window has been triggered.
  pub triggered: bool,
  /// Last watermark that affected this window.
  pub last_watermark: Option<Watermark>,
}

impl<W: Window, T: Clone> WindowState<W, T> {
  /// Creates a new window state.
  #[must_use]
  pub fn new(window: W) -> Self {
    Self {
      window,
      elements: Vec::new(),
      count: 0,
      triggered: false,
      last_watermark: None,
    }
  }

  /// Adds an element to the window.
  pub fn add(&mut self, timestamp: DateTime<Utc>, element: T) {
    self.elements.push((timestamp, element));
    self.count += 1;
  }

  /// Updates the watermark for this window.
  pub fn update_watermark(&mut self, watermark: Watermark) {
    self.last_watermark = Some(watermark);
  }

  /// Returns true if the window should be garbage collected.
  ///
  /// A window can be GC'd if it has been triggered and the watermark
  /// has passed the window's end time plus allowed lateness.
  #[must_use]
  pub fn can_be_gc(&self, current_watermark: &Watermark, allowed_lateness: ChronoDuration) -> bool {
    if !self.triggered {
      return false;
    }
    match self.window.end_time() {
      Some(end_time) => current_watermark.timestamp > end_time + allowed_lateness,
      None => false, // Global windows are never GC'd
    }
  }

  /// Clears all elements from the window.
  pub fn clear(&mut self) {
    self.elements.clear();
    self.count = 0;
  }

  /// Marks the window as triggered.
  pub fn mark_triggered(&mut self) {
    self.triggered = true;
  }

  /// Returns the elements as a slice.
  #[must_use]
  pub fn elements(&self) -> &[(DateTime<Utc>, T)] {
    &self.elements
  }

  /// Extracts the element values without timestamps.
  #[must_use]
  pub fn values(&self) -> Vec<T> {
    self.elements.iter().map(|(_, v)| v.clone()).collect()
  }
}

impl<W: Window, T: Clone> Clone for WindowState<W, T> {
  fn clone(&self) -> Self {
    Self {
      window: self.window.clone(),
      elements: self.elements.clone(),
      count: self.count,
      triggered: self.triggered,
      last_watermark: self.last_watermark.clone(),
    }
  }
}

/// Shared window assigner reference.
pub type SharedWindowAssigner<W> = Arc<dyn WindowAssigner<W = W>>;

#[cfg(test)]
mod tests {
  use super::*;

  fn timestamp(hour: u32, minute: u32, second: u32) -> DateTime<Utc> {
    use chrono::TimeZone;
    Utc
      .with_ymd_and_hms(2024, 1, 1, hour, minute, second)
      .unwrap()
  }

  // TimeWindow tests
  #[test]
  fn test_time_window_basic() {
    let start = timestamp(10, 0, 0);
    let end = timestamp(10, 5, 0);
    let window = TimeWindow::new(start, end);

    assert_eq!(window.start(), start);
    assert_eq!(window.end(), end);
    assert_eq!(window.duration(), ChronoDuration::minutes(5));
  }

  #[test]
  fn test_time_window_contains() {
    let window = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));

    assert!(window.contains(timestamp(10, 0, 0))); // Start is inclusive
    assert!(window.contains(timestamp(10, 2, 30)));
    assert!(!window.contains(timestamp(10, 5, 0))); // End is exclusive
    assert!(!window.contains(timestamp(9, 59, 59)));
  }

  #[test]
  fn test_time_window_intersects() {
    let w1 = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));
    let w2 = TimeWindow::new(timestamp(10, 3, 0), timestamp(10, 8, 0));
    let w3 = TimeWindow::new(timestamp(10, 5, 0), timestamp(10, 10, 0));
    let w4 = TimeWindow::new(timestamp(10, 10, 0), timestamp(10, 15, 0));

    assert!(w1.intersects(&w2)); // Overlap
    assert!(!w1.intersects(&w3)); // Adjacent
    assert!(!w1.intersects(&w4)); // Disjoint
  }

  #[test]
  fn test_time_window_merge() {
    let w1 = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));
    let w2 = TimeWindow::new(timestamp(10, 3, 0), timestamp(10, 8, 0));
    let w3 = TimeWindow::new(timestamp(10, 10, 0), timestamp(10, 15, 0));

    let merged = w1.merge(&w2).unwrap();
    assert_eq!(merged.start(), timestamp(10, 0, 0));
    assert_eq!(merged.end(), timestamp(10, 8, 0));

    assert!(w1.merge(&w3).is_none()); // Can't merge disjoint
  }

  #[test]
  fn test_time_window_display() {
    let window = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));
    let s = format!("{}", window);
    assert!(s.contains("10:00:00"));
    assert!(s.contains("10:05:00"));
  }

  // CountWindow tests
  #[test]
  fn test_count_window() {
    let window = CountWindow::new(42, 100);
    assert_eq!(window.id(), 42);
    assert_eq!(window.max_count(), 100);
  }

  // SessionWindow tests
  #[test]
  fn test_session_window_from_element() {
    let ts = timestamp(10, 0, 0);
    let gap = Duration::from_secs(300); // 5 minutes
    let session = SessionWindow::from_element(ts, gap);

    assert_eq!(session.start(), ts);
    assert_eq!(session.gap(), gap);
    assert!(session.contains(ts));
  }

  #[test]
  fn test_session_window_extend() {
    let mut session = SessionWindow::from_element(timestamp(10, 0, 0), Duration::from_secs(300));

    session.extend(timestamp(10, 2, 0));
    assert_eq!(session.start(), timestamp(10, 0, 0));
    // End should be extended

    session.extend(timestamp(9, 58, 0)); // Earlier than start
    assert_eq!(session.start(), timestamp(9, 58, 0));
  }

  #[test]
  fn test_session_window_merge() {
    let s1 = SessionWindow::from_element(timestamp(10, 0, 0), Duration::from_secs(300));
    let s2 = SessionWindow::from_element(timestamp(10, 2, 0), Duration::from_secs(300));
    let s3 = SessionWindow::from_element(timestamp(10, 30, 0), Duration::from_secs(300));

    assert!(s1.should_merge(&s2)); // Overlapping
    assert!(!s1.should_merge(&s3)); // Too far apart

    let merged = s1.merge(&s2).unwrap();
    assert_eq!(merged.start(), timestamp(10, 0, 0));
  }

  // TumblingWindowAssigner tests
  #[test]
  fn test_tumbling_assigner() {
    let assigner = TumblingWindowAssigner::new(Duration::from_secs(300)); // 5 minute windows

    let windows = assigner.assign_windows(timestamp(10, 2, 30));
    assert_eq!(windows.len(), 1);

    // Window should start at 10:00:00
    let window = &windows[0];
    assert_eq!(window.start(), timestamp(10, 0, 0));
    assert_eq!(window.end(), timestamp(10, 5, 0));
  }

  #[test]
  fn test_tumbling_assigner_with_offset() {
    let assigner =
      TumblingWindowAssigner::new(Duration::from_secs(300)).with_offset(Duration::from_secs(60)); // 1 minute offset

    let windows = assigner.assign_windows(timestamp(10, 2, 30));
    assert_eq!(windows.len(), 1);

    // Window should start at 10:01:00 with offset
    let window = &windows[0];
    assert_eq!(window.start(), timestamp(10, 1, 0));
    assert_eq!(window.end(), timestamp(10, 6, 0));
  }

  // SlidingWindowAssigner tests
  #[test]
  fn test_sliding_assigner() {
    let assigner = SlidingWindowAssigner::new(
      Duration::from_secs(300), // 5 minute window
      Duration::from_secs(60),  // 1 minute slide
    );

    let windows = assigner.assign_windows(timestamp(10, 2, 30));

    // Element should be in multiple windows
    assert!(windows.len() > 1);

    // All windows should contain the timestamp
    for window in &windows {
      assert!(window.contains(timestamp(10, 2, 30)));
    }
  }

  // SessionWindowAssigner tests
  #[test]
  fn test_session_assigner() {
    let assigner = SessionWindowAssigner::new(Duration::from_secs(300)); // 5 minute gap

    let windows = assigner.assign_windows(timestamp(10, 0, 0));
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].gap(), Duration::from_secs(300));
  }

  // CountWindowAssigner tests
  #[test]
  fn test_count_assigner() {
    let assigner = CountWindowAssigner::new(3);

    // First three elements in window 0
    for _ in 0..3 {
      let windows = assigner.assign_windows(Utc::now());
      assert_eq!(windows[0].id(), 0);
    }

    // Fourth element starts window 1
    let windows = assigner.assign_windows(Utc::now());
    assert_eq!(windows[0].id(), 1);
  }

  // GlobalWindowAssigner tests
  #[test]
  fn test_global_assigner() {
    let assigner = GlobalWindowAssigner::new();

    let windows = assigner.assign_windows(Utc::now());
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0], GlobalWindow);
  }

  // Trigger tests
  #[test]
  fn test_event_time_trigger() {
    let mut trigger = EventTimeTrigger::new();
    let window = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));

    // Before window end
    let result = trigger.on_event_time(timestamp(10, 3, 0), &window);
    assert_eq!(result, TriggerResult::Continue);

    // At window end
    let result = trigger.on_event_time(timestamp(10, 5, 0), &window);
    assert_eq!(result, TriggerResult::FireAndPurge);
  }

  #[test]
  fn test_count_trigger() {
    let mut trigger = CountTrigger::new(3);
    let window = CountWindow::new(0, 3);

    // First two elements
    assert_eq!(
      trigger.on_element(Utc::now(), &window),
      TriggerResult::Continue
    );
    assert_eq!(
      trigger.on_element(Utc::now(), &window),
      TriggerResult::Continue
    );

    // Third element triggers
    assert_eq!(
      trigger.on_element(Utc::now(), &window),
      TriggerResult::FireAndPurge
    );
  }

  #[test]
  fn test_processing_time_trigger() {
    let mut trigger = ProcessingTimeTrigger::new(Duration::from_secs(60));
    let window = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));

    // First call at 10:00:00
    let result = trigger.on_processing_time(timestamp(10, 0, 0), &window);
    assert_eq!(result, TriggerResult::Fire);

    // Call at 10:00:30 - not yet 1 minute
    let result = trigger.on_processing_time(timestamp(10, 0, 30), &window);
    assert_eq!(result, TriggerResult::Continue);

    // Call at 10:01:00 - 1 minute elapsed
    let result = trigger.on_processing_time(timestamp(10, 1, 0), &window);
    assert_eq!(result, TriggerResult::Fire);
  }

  // WindowConfig tests
  #[test]
  fn test_window_config() {
    let config = WindowConfig::new()
      .with_late_data_policy(LateDataPolicy::SideOutput)
      .with_allowed_lateness(Duration::from_secs(60));

    assert_eq!(config.late_data_policy, LateDataPolicy::SideOutput);
    assert_eq!(config.allowed_lateness, Duration::from_secs(60));
  }

  // Error tests
  #[test]
  fn test_window_error_display() {
    let err = WindowError::InvalidConfig("bad config".to_string());
    assert!(err.to_string().contains("Invalid window config"));

    let err = WindowError::NotFound("window1".to_string());
    assert!(err.to_string().contains("not found"));

    let err = WindowError::WindowClosed("window1".to_string());
    assert!(err.to_string().contains("closed"));

    let err = WindowError::StateError("state issue".to_string());
    assert!(err.to_string().contains("State error"));
  }

  // TriggerResult tests
  #[test]
  fn test_trigger_result_equality() {
    assert_eq!(TriggerResult::Continue, TriggerResult::Continue);
    assert_eq!(TriggerResult::Fire, TriggerResult::Fire);
    assert_ne!(TriggerResult::Fire, TriggerResult::FireAndPurge);
  }

  // LateDataPolicy tests
  #[test]
  fn test_late_data_policy_default() {
    let policy = LateDataPolicy::default();
    assert_eq!(policy, LateDataPolicy::Drop);
  }

  // Clone trigger tests
  #[test]
  fn test_trigger_clone() {
    let trigger = EventTimeTrigger::new();
    let cloned = trigger.clone_trigger();
    // Verify the cloned trigger works
    let window = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));
    assert_eq!(
      cloned
        .clone_trigger()
        .on_event_time(timestamp(10, 5, 0), &window),
      TriggerResult::FireAndPurge
    );
  }

  // ==========================================================================
  // Watermark Tests
  // ==========================================================================

  #[test]
  fn test_watermark_basic() {
    let ts = timestamp(10, 30, 0);
    let wm = Watermark::new(ts);
    assert_eq!(wm.timestamp, ts);
    assert!(!wm.is_end_of_stream());
    assert!(format!("{}", wm).contains("Watermark"));
  }

  #[test]
  fn test_watermark_min_max() {
    let min_wm = Watermark::min();
    let max_wm = Watermark::max();

    assert!(min_wm.timestamp < max_wm.timestamp);
    assert!(!min_wm.is_end_of_stream());
    assert!(max_wm.is_end_of_stream());
    assert!(format!("{}", max_wm).contains("END"));
  }

  #[test]
  fn test_watermark_advance() {
    let mut wm = Watermark::new(timestamp(10, 0, 0));

    // Advancing to a later time should succeed
    assert!(wm.advance(timestamp(10, 5, 0)));
    assert_eq!(wm.timestamp, timestamp(10, 5, 0));

    // Advancing to an earlier time should not change it
    assert!(!wm.advance(timestamp(10, 3, 0)));
    assert_eq!(wm.timestamp, timestamp(10, 5, 0));

    // Advancing to the same time should not change it
    assert!(!wm.advance(timestamp(10, 5, 0)));
    assert_eq!(wm.timestamp, timestamp(10, 5, 0));
  }

  #[test]
  fn test_watermark_ordering() {
    let wm1 = Watermark::new(timestamp(10, 0, 0));
    let wm2 = Watermark::new(timestamp(10, 5, 0));
    let wm3 = Watermark::new(timestamp(10, 0, 0));

    assert!(wm1 < wm2);
    assert!(wm2 > wm1);
    assert_eq!(wm1, wm3);
  }

  // ==========================================================================
  // Watermark Generator Tests
  // ==========================================================================

  #[test]
  fn test_monotonic_generator() {
    let mut generator = MonotonicWatermarkGenerator::new();
    assert_eq!(generator.current_watermark(), Watermark::min());

    // First event should emit watermark
    let wm = generator.on_event(timestamp(10, 0, 0));
    assert!(wm.is_some());
    assert_eq!(wm.unwrap().timestamp, timestamp(10, 0, 0));

    // Later event should emit watermark
    let wm = generator.on_event(timestamp(10, 5, 0));
    assert!(wm.is_some());
    assert_eq!(wm.unwrap().timestamp, timestamp(10, 5, 0));

    // Earlier event should not emit watermark
    let wm = generator.on_event(timestamp(10, 3, 0));
    assert!(wm.is_none());
    assert_eq!(generator.current_watermark().timestamp, timestamp(10, 5, 0));

    // Periodic emit should return None
    assert!(generator.on_periodic_emit().is_none());
  }

  #[test]
  fn test_bounded_out_of_orderness_generator() {
    let max_delay = Duration::from_secs(5);
    let mut generator = BoundedOutOfOrdernessGenerator::new(max_delay);
    assert_eq!(generator.current_watermark(), Watermark::min());

    // First event at 10:00:00 -> watermark at 09:59:55
    let wm = generator.on_event(timestamp(10, 0, 0));
    assert!(wm.is_some());
    assert_eq!(wm.unwrap().timestamp, timestamp(9, 59, 55));

    // Event at 10:00:10 -> watermark at 10:00:05
    let wm = generator.on_event(timestamp(10, 0, 10));
    assert!(wm.is_some());
    assert_eq!(wm.unwrap().timestamp, timestamp(10, 0, 5));

    // Event at 10:00:05 (late but within bounds) -> no new watermark
    let wm = generator.on_event(timestamp(10, 0, 5));
    assert!(wm.is_none());
    assert_eq!(generator.current_watermark().timestamp, timestamp(10, 0, 5));

    // Periodic emit should return current watermark
    let wm = generator.on_periodic_emit();
    assert!(wm.is_some());
    assert_eq!(wm.unwrap().timestamp, timestamp(10, 0, 5));
  }

  #[test]
  fn test_periodic_watermark_generator() {
    let max_delay = Duration::from_secs(5);
    let interval = Duration::from_secs(10);
    let mut generator = PeriodicWatermarkGenerator::new(max_delay, interval);

    // Events should not emit watermarks directly
    let wm = generator.on_event(timestamp(10, 0, 0));
    assert!(wm.is_none());

    generator.on_event(timestamp(10, 0, 10));
    assert!(generator.on_event(timestamp(10, 0, 15)).is_none());

    // First periodic emit should work (no previous emit)
    let wm = generator.on_periodic_emit();
    assert!(wm.is_some());
    // Watermark should be 10:00:10 (max) - 5s (delay) = 10:00:05
    assert_eq!(wm.unwrap().timestamp, timestamp(10, 0, 10));
  }

  #[test]
  fn test_watermark_generator_end_of_stream() {
    let mut generator = MonotonicWatermarkGenerator::new();
    generator.on_event(timestamp(10, 0, 0));

    let eos = generator.on_end_of_stream();
    assert!(eos.is_end_of_stream());
  }

  // ==========================================================================
  // Late Data Handler Tests
  // ==========================================================================

  #[test]
  fn test_late_data_result() {
    let on_time: LateDataResult<i32> = LateDataResult::OnTime(42);
    assert!(on_time.should_process());
    assert_eq!(on_time.into_processable(), Some(42));

    let within: LateDataResult<i32> = LateDataResult::WithinLateness(42);
    assert!(within.should_process());

    let drop: LateDataResult<i32> = LateDataResult::Drop;
    assert!(!drop.should_process());
    assert_eq!(drop.into_processable(), None);

    let side: LateDataResult<i32> = LateDataResult::SideOutput(42);
    assert!(!side.should_process());
    assert!(side.is_side_output());
    assert_eq!(LateDataResult::SideOutput(42).into_side_output(), Some(42));
  }

  #[test]
  fn test_late_data_handler_on_time() {
    let mut handler = LateDataHandler::drop_late();
    let watermark = Watermark::new(timestamp(10, 0, 0));

    // Element at same time as watermark is on-time
    let result = handler.evaluate(timestamp(10, 0, 0), &watermark, "data");
    assert!(matches!(result, LateDataResult::OnTime("data")));

    // Element after watermark is on-time
    let result = handler.evaluate(timestamp(10, 5, 0), &watermark, "data");
    assert!(matches!(result, LateDataResult::OnTime("data")));

    assert_eq!(handler.stats().on_time, 2);
  }

  #[test]
  fn test_late_data_handler_within_lateness() {
    let mut handler = LateDataHandler::with_allowed_lateness(Duration::from_secs(30));
    let watermark = Watermark::new(timestamp(10, 0, 0));

    // Element 10 seconds before watermark (within 30s lateness)
    let result = handler.evaluate(timestamp(9, 59, 50), &watermark, "data");
    assert!(matches!(result, LateDataResult::WithinLateness("data")));

    assert_eq!(handler.stats().within_lateness, 1);
  }

  #[test]
  fn test_late_data_handler_drop() {
    let mut handler = LateDataHandler::drop_late();
    let watermark = Watermark::new(timestamp(10, 0, 0));

    // Element 30 seconds before watermark (late, no allowed lateness)
    let result = handler.evaluate(timestamp(9, 59, 30), &watermark, "data");
    assert!(matches!(result, LateDataResult::Drop));

    assert_eq!(handler.stats().dropped, 1);
  }

  #[test]
  fn test_late_data_handler_side_output() {
    let mut handler = LateDataHandler::redirect_to_side_output();
    let watermark = Watermark::new(timestamp(10, 0, 0));

    // Element 30 seconds before watermark (late, goes to side output)
    let result = handler.evaluate(timestamp(9, 59, 30), &watermark, "data");
    assert!(matches!(result, LateDataResult::SideOutput("data")));

    assert_eq!(handler.stats().side_output, 1);
  }

  #[test]
  fn test_late_data_handler_stats_reset() {
    let mut handler = LateDataHandler::drop_late();
    let watermark = Watermark::new(timestamp(10, 0, 0));

    handler.evaluate(timestamp(10, 0, 0), &watermark, "data");
    handler.evaluate(timestamp(9, 59, 0), &watermark, "data");

    assert_eq!(handler.stats().on_time, 1);
    assert_eq!(handler.stats().dropped, 1);

    handler.reset_stats();
    assert_eq!(handler.stats().on_time, 0);
    assert_eq!(handler.stats().dropped, 0);
  }

  #[test]
  fn test_late_data_handler_accessors() {
    let handler = LateDataHandler::with_allowed_lateness(Duration::from_secs(30));
    assert!(matches!(handler.policy(), LateDataPolicy::AllowLateness(_)));
    assert_eq!(handler.get_allowed_lateness(), Duration::from_secs(30));
  }

  // ==========================================================================
  // Window State Tests
  // ==========================================================================

  #[test]
  fn test_window_state_basic() {
    let window = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));
    let mut state: WindowState<TimeWindow, i32> = WindowState::new(window.clone());

    assert_eq!(state.count, 0);
    assert!(!state.triggered);
    assert!(state.last_watermark.is_none());

    state.add(timestamp(10, 1, 0), 42);
    state.add(timestamp(10, 2, 0), 43);

    assert_eq!(state.count, 2);
    assert_eq!(state.elements().len(), 2);
    assert_eq!(state.values(), vec![42, 43]);
  }

  #[test]
  fn test_window_state_watermark() {
    let window = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));
    let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

    let wm = Watermark::new(timestamp(10, 3, 0));
    state.update_watermark(wm.clone());
    assert_eq!(state.last_watermark, Some(wm));
  }

  #[test]
  fn test_window_state_trigger_and_clear() {
    let window = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));
    let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

    state.add(timestamp(10, 1, 0), 42);
    state.mark_triggered();
    assert!(state.triggered);

    state.clear();
    assert_eq!(state.count, 0);
    assert!(state.elements().is_empty());
  }

  #[test]
  fn test_window_state_gc() {
    let window = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));
    let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

    // Non-triggered windows cannot be GC'd
    let wm = Watermark::new(timestamp(11, 0, 0));
    assert!(!state.can_be_gc(&wm, ChronoDuration::zero()));

    // Triggered window can be GC'd after watermark passes end + lateness
    state.mark_triggered();
    assert!(state.can_be_gc(&wm, ChronoDuration::zero()));

    // With allowed lateness, needs more time
    let wm2 = Watermark::new(timestamp(10, 5, 0));
    assert!(!state.can_be_gc(&wm2, ChronoDuration::minutes(1)));
  }

  #[test]
  fn test_window_state_clone() {
    let window = TimeWindow::new(timestamp(10, 0, 0), timestamp(10, 5, 0));
    let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);
    state.add(timestamp(10, 1, 0), 42);
    state.mark_triggered();

    let cloned = state.clone();
    assert_eq!(cloned.count, 1);
    assert!(cloned.triggered);
    assert_eq!(cloned.values(), vec![42]);
  }
}
