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
//! - [`Window`]: Represents a window boundary (start/end time)
//! - [`WindowAssigner`]: Assigns elements to windows
//! - [`WindowTrigger`]: Determines when to emit window results
//! - [`LateDataPolicy`]: Handles elements arriving after window closes
//!
//! # Window Types
//!
//! - [`TumblingWindow`]: Fixed-size, non-overlapping windows
//! - [`SlidingWindow`]: Fixed-size, overlapping windows
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
use std::fmt::{self, Debug};
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
}
