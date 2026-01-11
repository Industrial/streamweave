//! # Window Test Suite
//!
//! Comprehensive test suite for windowing operations, including time windows,
//! count windows, session windows, triggers, assigners, and watermark generation.
//!
//! ## Test Coverage
//!
//! This test suite covers:
//!
//! - **Window Types**: TimeWindow, CountWindow, SessionWindow, and GlobalWindow
//! - **Window Operations**: Window creation, merging, intersection, and containment checks
//! - **Triggers**: EventTimeTrigger, CountTrigger, SessionTrigger, ProcessingTimeTrigger, NeverTrigger
//! - **Window Assigners**: Tumbling, Sliding, Session, Count, and Global window assigners
//! - **Watermark Generation**: Monotonic, BoundedOutOfOrderness, and Periodic watermark generators
//! - **Late Data Handling**: Late data policies and evaluation
//! - **Window State**: State management for windowed operations
//! - **Window Configuration**: Late data policies and allowed lateness settings
//!
//! ## Test Organization
//!
//! Tests are organized by component:
//!
//! - Trigger result tests
//! - Late data policy tests
//! - Window error tests
//! - Time window tests
//! - Count window tests
//! - Session window tests
//! - Trigger tests (EventTime, Count, Session, ProcessingTime, Never)
//! - Window assigner tests (Tumbling, Sliding, Session, Count, Global)
//! - Window config tests
//! - Watermark tests
//! - Watermark generator tests
//! - Late data handler tests
//! - Window state tests

use crate::window::*;
use chrono::{DateTime, Utc};
use std::time::Duration;

// ============================================================================
// TriggerResult Tests
// ============================================================================

#[test]
fn test_trigger_result_variants() {
  assert_eq!(TriggerResult::Continue, TriggerResult::Continue);
  assert_eq!(TriggerResult::Fire, TriggerResult::Fire);
  assert_eq!(TriggerResult::FireAndPurge, TriggerResult::FireAndPurge);
  assert_eq!(TriggerResult::Purge, TriggerResult::Purge);
}

#[test]
fn test_trigger_result_clone() {
  let result = TriggerResult::Fire;
  assert_eq!(result, result.clone());
}

#[test]
fn test_trigger_result_debug() {
  let result = TriggerResult::Continue;
  let _ = format!("{:?}", result);
}

// ============================================================================
// LateDataPolicy Tests
// ============================================================================

#[test]
fn test_late_data_policy_default() {
  assert_eq!(LateDataPolicy::default(), LateDataPolicy::Drop);
}

#[test]
fn test_late_data_policy_variants() {
  let drop = LateDataPolicy::Drop;
  let side_output = LateDataPolicy::SideOutput;
  let allow = LateDataPolicy::AllowLateness(Duration::from_secs(5));

  assert_eq!(drop, LateDataPolicy::Drop);
  assert_eq!(side_output, LateDataPolicy::SideOutput);
  assert_eq!(allow, LateDataPolicy::AllowLateness(Duration::from_secs(5)));
}

#[test]
fn test_late_data_policy_clone() {
  let policy = LateDataPolicy::AllowLateness(Duration::from_secs(10));
  assert_eq!(policy, policy.clone());
}

#[test]
fn test_late_data_policy_debug() {
  let policy = LateDataPolicy::Drop;
  let _ = format!("{:?}", policy);
}

// ============================================================================
// WindowError Tests
// ============================================================================

#[test]
fn test_window_error_display() {
  let invalid_config = WindowError::InvalidConfig("test".to_string());
  assert_eq!(format!("{}", invalid_config), "Invalid window config: test");

  let not_found = WindowError::NotFound("window1".to_string());
  assert_eq!(format!("{}", not_found), "Window not found: window1");

  let closed = WindowError::WindowClosed("window2".to_string());
  assert_eq!(format!("{}", closed), "Window closed: window2");

  let state_error = WindowError::StateError("error".to_string());
  assert_eq!(format!("{}", state_error), "State error: error");
}

#[test]
fn test_window_error_debug() {
  let err = WindowError::InvalidConfig("test".to_string());
  let _ = format!("{:?}", err);
}

#[test]
fn test_window_error_error_trait() {
  let err: Box<dyn std::error::Error> = Box::new(WindowError::InvalidConfig("test".to_string()));
  let _ = format!("{}", err);
}

// ============================================================================
// TimeWindow Tests
// ============================================================================

#[test]
fn test_time_window_new() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);
  assert_eq!(window.start(), start);
  assert_eq!(window.end(), end);
}

#[test]
fn test_time_window_duration() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(5);
  let window = TimeWindow::new(start, end);
  assert_eq!(window.duration(), chrono::Duration::seconds(5));
}

#[test]
fn test_time_window_max_timestamp() {
  let start = Utc::now();
  let end = start + chrono::Duration::milliseconds(1000);
  let window = TimeWindow::new(start, end);
  let max_ts = window.max_timestamp();
  assert!(max_ts < end);
  assert!(max_ts >= start);
}

#[test]
fn test_time_window_contains() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  assert!(window.contains(start));
  assert!(window.contains(start + chrono::Duration::seconds(5)));
  assert!(!window.contains(end));
  assert!(!window.contains(start - chrono::Duration::seconds(1)));
}

#[test]
fn test_time_window_intersects() {
  let start1 = Utc::now();
  let end1 = start1 + chrono::Duration::seconds(10);
  let window1 = TimeWindow::new(start1, end1);

  let start2 = start1 + chrono::Duration::seconds(5);
  let end2 = start2 + chrono::Duration::seconds(10);
  let window2 = TimeWindow::new(start2, end2);

  assert!(window1.intersects(&window2));
  assert!(window2.intersects(&window1));

  let start3 = end1 + chrono::Duration::seconds(1);
  let end3 = start3 + chrono::Duration::seconds(10);
  let window3 = TimeWindow::new(start3, end3);

  assert!(!window1.intersects(&window3));
}

#[test]
fn test_time_window_merge() {
  let start1 = Utc::now();
  let end1 = start1 + chrono::Duration::seconds(10);
  let window1 = TimeWindow::new(start1, end1);

  let start2 = start1 + chrono::Duration::seconds(5);
  let end2 = start2 + chrono::Duration::seconds(10);
  let window2 = TimeWindow::new(start2, end2);

  let merged = window1.merge(&window2).unwrap();
  assert_eq!(merged.start(), start1.min(start2));
  assert_eq!(merged.end(), end1.max(end2));

  let start3 = end1 + chrono::Duration::seconds(2);
  let end3 = start3 + chrono::Duration::seconds(10);
  let window3 = TimeWindow::new(start3, end3);

  assert!(window1.merge(&window3).is_none());
}

#[test]
fn test_time_window_merge_adjacent() {
  let start1 = Utc::now();
  let end1 = start1 + chrono::Duration::seconds(10);
  let window1 = TimeWindow::new(start1, end1);

  let start2 = end1;
  let end2 = start2 + chrono::Duration::seconds(10);
  let window2 = TimeWindow::new(start2, end2);

  let merged = window1.merge(&window2).unwrap();
  assert_eq!(merged.start(), start1);
  assert_eq!(merged.end(), end2);
}

#[test]
fn test_time_window_eq() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window1 = TimeWindow::new(start, end);
  let window2 = TimeWindow::new(start, end);
  assert_eq!(window1, window2);
}

#[test]
fn test_time_window_hash() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window1 = TimeWindow::new(start, end);
  let window2 = TimeWindow::new(start, end);
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher1 = DefaultHasher::new();
  let mut hasher2 = DefaultHasher::new();
  window1.hash(&mut hasher1);
  window2.hash(&mut hasher2);
  assert_eq!(hasher1.finish(), hasher2.finish());
}

#[test]
fn test_time_window_ord() {
  let start1 = Utc::now();
  let end1 = start1 + chrono::Duration::seconds(10);
  let window1 = TimeWindow::new(start1, end1);

  let start2 = start1 + chrono::Duration::seconds(5);
  let end2 = start2 + chrono::Duration::seconds(10);
  let window2 = TimeWindow::new(start2, end2);

  assert!(window1 < window2);
}

#[test]
fn test_time_window_display() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);
  let _ = format!("{}", window);
}

#[test]
fn test_time_window_trait() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  assert_eq!(window.end_time(), Some(end));
  assert_eq!(window.start_time(), Some(start));
  assert!(window.contains(start + chrono::Duration::seconds(5)));
}

// ============================================================================
// CountWindow Tests
// ============================================================================

#[test]
fn test_count_window_new() {
  let window = CountWindow::new(1, 10);
  assert_eq!(window.id(), 1);
  assert_eq!(window.max_count(), 10);
}

#[test]
fn test_count_window_eq() {
  let window1 = CountWindow::new(1, 10);
  let window2 = CountWindow::new(1, 10);
  assert_eq!(window1, window2);
}

#[test]
fn test_count_window_hash() {
  let window1 = CountWindow::new(1, 10);
  let window2 = CountWindow::new(1, 10);
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher1 = DefaultHasher::new();
  let mut hasher2 = DefaultHasher::new();
  window1.hash(&mut hasher1);
  window2.hash(&mut hasher2);
  assert_eq!(hasher1.finish(), hasher2.finish());
}

#[test]
fn test_count_window_display() {
  let window = CountWindow::new(42, 100);
  let display = format!("{}", window);
  assert!(display.contains("42"));
  assert!(display.contains("100"));
}

#[test]
fn test_count_window_clone() {
  let window1 = CountWindow::new(1, 10);
  let window2 = window1.clone();
  assert_eq!(window1, window2);
}

// ============================================================================
// SessionWindow Tests
// ============================================================================

#[test]
fn test_session_window_new() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window = SessionWindow::new(start, end, gap);
  assert_eq!(window.start(), start);
  assert_eq!(window.end(), end);
  assert_eq!(window.gap(), gap);
}

#[test]
fn test_session_window_from_element() {
  let timestamp = Utc::now();
  let gap = Duration::from_secs(5);
  let window = SessionWindow::from_element(timestamp, gap);
  assert_eq!(window.start(), timestamp);
  assert!(window.end() >= timestamp);
  assert_eq!(window.gap(), gap);
}

#[test]
fn test_session_window_contains() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window = SessionWindow::new(start, end, gap);

  assert!(window.contains(start));
  assert!(window.contains(start + chrono::Duration::seconds(5)));
  assert!(!window.contains(end));
}

#[test]
fn test_session_window_should_merge() {
  let start1 = Utc::now();
  let end1 = start1 + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window1 = SessionWindow::new(start1, end1, gap);

  let start2 = start1 + chrono::Duration::seconds(5);
  let end2 = start2 + chrono::Duration::seconds(10);
  let window2 = SessionWindow::new(start2, end2, gap);

  assert!(window1.should_merge(&window2));

  let start3 = end1 + chrono::Duration::seconds(1);
  let end3 = start3 + chrono::Duration::seconds(10);
  let window3 = SessionWindow::new(start3, end3, gap);

  assert!(!window1.should_merge(&window3));
}

#[test]
fn test_session_window_merge() {
  let start1 = Utc::now();
  let end1 = start1 + chrono::Duration::seconds(10);
  let gap1 = Duration::from_secs(5);
  let window1 = SessionWindow::new(start1, end1, gap1);

  let start2 = start1 + chrono::Duration::seconds(5);
  let end2 = start2 + chrono::Duration::seconds(15);
  let gap2 = Duration::from_secs(7);
  let window2 = SessionWindow::new(start2, end2, gap2);

  let merged = window1.merge(&window2).unwrap();
  assert_eq!(merged.start(), start1.min(start2));
  assert_eq!(merged.end(), end1.max(end2));
  assert_eq!(merged.gap(), gap1.max(gap2));
}

#[test]
fn test_session_window_extend() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let mut window = SessionWindow::new(start, end, gap);

  let new_timestamp = start + chrono::Duration::seconds(12);
  window.extend(new_timestamp);

  assert!(window.end() >= new_timestamp);
}

#[test]
fn test_session_window_extend_earlier() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let mut window = SessionWindow::new(start, end, gap);

  let earlier_timestamp = start - chrono::Duration::seconds(2);
  window.extend(earlier_timestamp);

  assert_eq!(window.start(), earlier_timestamp);
}

#[test]
fn test_session_window_eq() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window1 = SessionWindow::new(start, end, gap);
  let window2 = SessionWindow::new(start, end, gap);
  assert_eq!(window1, window2);
}

#[test]
fn test_session_window_hash() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window1 = SessionWindow::new(start, end, gap);
  let window2 = SessionWindow::new(start, end, gap);
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher1 = DefaultHasher::new();
  let mut hasher2 = DefaultHasher::new();
  window1.hash(&mut hasher1);
  window2.hash(&mut hasher2);
  assert_eq!(hasher1.finish(), hasher2.finish());
}

#[test]
fn test_session_window_display() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window = SessionWindow::new(start, end, gap);
  let _ = format!("{}", window);
}

#[test]
fn test_session_window_clone() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window1 = SessionWindow::new(start, end, gap);
  let window2 = window1.clone();
  assert_eq!(window1, window2);
}

// ============================================================================
// EventTimeTrigger Tests
// ============================================================================

#[test]
fn test_event_time_trigger_new() {
  let _trigger = EventTimeTrigger::new();
  // Just verify it can be created
}

#[test]
fn test_event_time_trigger_default() {
  let _trigger = EventTimeTrigger;
  // Just verify it can be created
}

#[test]
fn test_event_time_trigger_on_element() {
  let mut trigger = EventTimeTrigger::new();
  let timestamp = Utc::now();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  let result = trigger.on_element(timestamp, &window);
  assert_eq!(result, TriggerResult::Continue);
}

#[test]
fn test_event_time_trigger_on_processing_time() {
  let mut trigger = EventTimeTrigger::new();
  let time = Utc::now();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  let result = trigger.on_processing_time(time, &window);
  assert_eq!(result, TriggerResult::Continue);
}

#[test]
fn test_event_time_trigger_on_event_time() {
  let mut trigger = EventTimeTrigger::new();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  // Watermark before window end
  let watermark = start + chrono::Duration::seconds(5);
  let result = trigger.on_event_time(watermark, &window);
  assert_eq!(result, TriggerResult::Continue);

  // Watermark at window end
  let result = trigger.on_event_time(end, &window);
  assert_eq!(result, TriggerResult::FireAndPurge);

  // Watermark after window end
  let watermark = end + chrono::Duration::seconds(1);
  let result = trigger.on_event_time(watermark, &window);
  assert_eq!(result, TriggerResult::FireAndPurge);
}

#[test]
fn test_event_time_trigger_clear() {
  let mut trigger = EventTimeTrigger::new();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  // Should not panic
  trigger.clear(&window);
}

#[test]
fn test_event_time_trigger_clone() {
  let mut trigger = EventTimeTrigger::new();
  let mut cloned = trigger.clone_trigger();
  let timestamp = Utc::now();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  assert_eq!(
    trigger.on_element(timestamp, &window),
    cloned.on_element(timestamp, &window)
  );
}

// ============================================================================
// CountTrigger Tests
// ============================================================================

#[test]
fn test_count_trigger_new() {
  let trigger = CountTrigger::new(10);
  assert_eq!(trigger.count(), 10);
}

#[test]
fn test_count_trigger_on_element() {
  let mut trigger = CountTrigger::new(3);
  let timestamp = Utc::now();
  let window1 = CountWindow::new(1, 10);
  let window2 = CountWindow::new(2, 10);

  // First two elements
  assert_eq!(
    trigger.on_element(timestamp, &window1),
    TriggerResult::Continue
  );
  assert_eq!(
    trigger.on_element(timestamp, &window1),
    TriggerResult::Continue
  );

  // Third element fires
  assert_eq!(
    trigger.on_element(timestamp, &window1),
    TriggerResult::FireAndPurge
  );

  // Different window starts from 0
  assert_eq!(
    trigger.on_element(timestamp, &window2),
    TriggerResult::Continue
  );
}

#[test]
fn test_count_trigger_on_processing_time() {
  let mut trigger = CountTrigger::new(10);
  let time = Utc::now();
  let window = CountWindow::new(1, 10);

  let result = trigger.on_processing_time(time, &window);
  assert_eq!(result, TriggerResult::Continue);
}

#[test]
fn test_count_trigger_on_event_time() {
  let mut trigger = CountTrigger::new(10);
  let watermark = Utc::now();
  let window = CountWindow::new(1, 10);

  let result = trigger.on_event_time(watermark, &window);
  assert_eq!(result, TriggerResult::Continue);
}

#[test]
fn test_count_trigger_clear() {
  let mut trigger = CountTrigger::new(10);
  let timestamp = Utc::now();
  let window = CountWindow::new(1, 10);

  trigger.on_element(timestamp, &window);
  trigger.clear(&window);

  // After clear, count should reset
  assert_eq!(
    trigger.on_element(timestamp, &window),
    TriggerResult::Continue
  );
}

#[test]
fn test_count_trigger_clone() {
  let mut trigger = CountTrigger::new(5);
  let mut cloned = trigger.clone_trigger();
  let timestamp = Utc::now();
  let window = CountWindow::new(1, 10);

  // Both should start fresh
  assert_eq!(
    trigger.on_element(timestamp, &window),
    cloned.on_element(timestamp, &window)
  );
}

// ============================================================================
// SessionTrigger Tests
// ============================================================================

#[test]
fn test_session_trigger_new() {
  let _trigger = SessionTrigger::new();
  // Just verify it can be created
}

#[test]
fn test_session_trigger_default() {
  let _trigger = SessionTrigger::default();
  // Just verify it can be created
}

#[test]
fn test_session_trigger_on_element() {
  let mut trigger = SessionTrigger::new();
  let timestamp = Utc::now();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window = SessionWindow::new(start, end, gap);

  let result = trigger.on_element(timestamp, &window);
  assert_eq!(result, TriggerResult::Continue);
}

#[test]
fn test_session_trigger_on_processing_time() {
  let mut trigger = SessionTrigger::new();
  let time = Utc::now();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window = SessionWindow::new(start, end, gap);

  let result = trigger.on_processing_time(time, &window);
  assert_eq!(result, TriggerResult::Continue);
}

#[test]
fn test_session_trigger_on_event_time() {
  let mut trigger = SessionTrigger::new();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window = SessionWindow::new(start, end, gap);

  // Watermark before window end
  let watermark = start + chrono::Duration::seconds(5);
  let result = trigger.on_event_time(watermark, &window);
  assert_eq!(result, TriggerResult::Continue);

  // Watermark at window end
  let result = trigger.on_event_time(end, &window);
  assert_eq!(result, TriggerResult::FireAndPurge);

  // Watermark after window end
  let watermark = end + chrono::Duration::seconds(1);
  let result = trigger.on_event_time(watermark, &window);
  assert_eq!(result, TriggerResult::FireAndPurge);
}

#[test]
fn test_session_trigger_clear() {
  let mut trigger = SessionTrigger::new();
  let timestamp = Utc::now();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window = SessionWindow::new(start, end, gap);

  trigger.on_element(timestamp, &window);
  trigger.clear(&window);

  // After clear, should work normally
  assert_eq!(
    trigger.on_element(timestamp, &window),
    TriggerResult::Continue
  );
}

#[test]
fn test_session_trigger_clone() {
  let mut trigger = SessionTrigger::new();
  let mut cloned = trigger.clone_trigger();
  let timestamp = Utc::now();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let gap = Duration::from_secs(5);
  let window = SessionWindow::new(start, end, gap);

  assert_eq!(
    trigger.on_element(timestamp, &window),
    cloned.on_element(timestamp, &window)
  );
}

// ============================================================================
// ProcessingTimeTrigger Tests
// ============================================================================

#[test]
fn test_processing_time_trigger_new() {
  let interval = Duration::from_secs(5);
  let trigger = ProcessingTimeTrigger::new(interval);
  assert_eq!(trigger.interval(), interval);
}

#[test]
fn test_processing_time_trigger_on_element() {
  let interval = Duration::from_secs(5);
  let mut trigger = ProcessingTimeTrigger::new(interval);
  let timestamp = Utc::now();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  let result = trigger.on_element(timestamp, &window);
  assert_eq!(result, TriggerResult::Continue);
}

#[test]
fn test_processing_time_trigger_on_processing_time() {
  let interval = Duration::from_secs(5);
  let mut trigger = ProcessingTimeTrigger::new(interval);
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  // First call should fire (no previous fire time)
  let time1 = Utc::now();
  let result = trigger.on_processing_time(time1, &window);
  assert_eq!(result, TriggerResult::Fire);

  // Immediate call should not fire
  let result = trigger.on_processing_time(time1, &window);
  assert_eq!(result, TriggerResult::Continue);

  // After interval, should fire
  let time2 =
    time1 + chrono::Duration::from_std(interval).unwrap() + chrono::Duration::milliseconds(1);
  let result = trigger.on_processing_time(time2, &window);
  assert_eq!(result, TriggerResult::Fire);
}

#[test]
fn test_processing_time_trigger_on_event_time() {
  let interval = Duration::from_secs(5);
  let mut trigger = ProcessingTimeTrigger::new(interval);
  let watermark = Utc::now();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  let result = trigger.on_event_time(watermark, &window);
  assert_eq!(result, TriggerResult::Continue);
}

#[test]
fn test_processing_time_trigger_clear() {
  let interval = Duration::from_secs(5);
  let mut trigger = ProcessingTimeTrigger::new(interval);
  let time = Utc::now();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  trigger.on_processing_time(time, &window);
  trigger.clear(&window);

  // After clear, should fire on next call
  let result = trigger.on_processing_time(time, &window);
  assert_eq!(result, TriggerResult::Fire);
}

#[test]
fn test_processing_time_trigger_clone() {
  let interval = Duration::from_secs(5);
  let mut trigger = ProcessingTimeTrigger::new(interval);
  let mut cloned = trigger.clone_trigger();
  let timestamp = Utc::now();
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  assert_eq!(
    trigger.on_element(timestamp, &window),
    cloned.on_element(timestamp, &window)
  );
}

// ============================================================================
// TumblingWindowAssigner Tests
// ============================================================================

#[test]
fn test_tumbling_window_assigner_new() {
  let size = Duration::from_secs(10);
  let assigner = TumblingWindowAssigner::new(size);
  assert_eq!(assigner.size(), size);
  assert_eq!(assigner.offset(), Duration::ZERO);
}

#[test]
fn test_tumbling_window_assigner_with_offset() {
  let size = Duration::from_secs(10);
  let offset = Duration::from_secs(5);
  let assigner = TumblingWindowAssigner::new(size).with_offset(offset);
  assert_eq!(assigner.size(), size);
  assert_eq!(assigner.offset(), offset);
}

#[test]
fn test_tumbling_window_assigner_assign_windows() {
  let size = Duration::from_secs(10);
  let assigner = TumblingWindowAssigner::new(size);
  let timestamp = DateTime::from_timestamp(1000, 0).unwrap();

  let windows = assigner.assign_windows(timestamp);
  assert_eq!(windows.len(), 1);

  let window = &windows[0];
  assert!(window.contains(timestamp));
  assert_eq!(window.duration(), chrono::Duration::from_std(size).unwrap());
}

#[test]
fn test_tumbling_window_assigner_default_trigger() {
  let size = Duration::from_secs(10);
  let assigner = TumblingWindowAssigner::new(size);
  let _trigger = assigner.default_trigger();
}

#[test]
fn test_tumbling_window_assigner_is_event_time() {
  let size = Duration::from_secs(10);
  let assigner = TumblingWindowAssigner::new(size);
  assert!(assigner.is_event_time());
}

#[test]
fn test_tumbling_window_assigner_clone() {
  let size = Duration::from_secs(10);
  let assigner1 = TumblingWindowAssigner::new(size);
  let assigner2 = assigner1.clone();
  assert_eq!(assigner1.size(), assigner2.size());
  assert_eq!(assigner1.offset(), assigner2.offset());
}

// ============================================================================
// SlidingWindowAssigner Tests
// ============================================================================

#[test]
fn test_sliding_window_assigner_new() {
  let size = Duration::from_secs(10);
  let slide = Duration::from_secs(5);
  let assigner = SlidingWindowAssigner::new(size, slide);
  assert_eq!(assigner.size(), size);
  assert_eq!(assigner.slide(), slide);
  assert_eq!(assigner.offset(), Duration::ZERO);
}

#[test]
fn test_sliding_window_assigner_with_offset() {
  let size = Duration::from_secs(10);
  let slide = Duration::from_secs(5);
  let offset = Duration::from_secs(2);
  let assigner = SlidingWindowAssigner::new(size, slide).with_offset(offset);
  assert_eq!(assigner.size(), size);
  assert_eq!(assigner.slide(), slide);
  assert_eq!(assigner.offset(), offset);
}

#[test]
fn test_sliding_window_assigner_assign_windows() {
  let size = Duration::from_secs(10);
  let slide = Duration::from_secs(5);
  let assigner = SlidingWindowAssigner::new(size, slide);
  let timestamp = DateTime::from_timestamp(1000, 0).unwrap();

  let windows = assigner.assign_windows(timestamp);
  // Should get multiple windows (size / slide)
  assert!(windows.len() > 1);

  // All windows should contain the timestamp
  for window in &windows {
    assert!(window.contains(timestamp));
  }
}

#[test]
fn test_sliding_window_assigner_default_trigger() {
  let size = Duration::from_secs(10);
  let slide = Duration::from_secs(5);
  let assigner = SlidingWindowAssigner::new(size, slide);
  let _trigger = assigner.default_trigger();
}

#[test]
fn test_sliding_window_assigner_is_event_time() {
  let size = Duration::from_secs(10);
  let slide = Duration::from_secs(5);
  let assigner = SlidingWindowAssigner::new(size, slide);
  assert!(assigner.is_event_time());
}

#[test]
fn test_sliding_window_assigner_clone() {
  let size = Duration::from_secs(10);
  let slide = Duration::from_secs(5);
  let assigner1 = SlidingWindowAssigner::new(size, slide);
  let assigner2 = assigner1.clone();
  assert_eq!(assigner1.size(), assigner2.size());
  assert_eq!(assigner1.slide(), assigner2.slide());
  assert_eq!(assigner1.offset(), assigner2.offset());
}

// ============================================================================
// SessionWindowAssigner Tests
// ============================================================================

#[test]
fn test_session_window_assigner_new() {
  let gap = Duration::from_secs(5);
  let assigner = SessionWindowAssigner::new(gap);
  assert_eq!(assigner.gap(), gap);
}

#[test]
fn test_session_window_assigner_assign_windows() {
  let gap = Duration::from_secs(5);
  let assigner = SessionWindowAssigner::new(gap);
  let timestamp = Utc::now();

  let windows = assigner.assign_windows(timestamp);
  assert_eq!(windows.len(), 1);

  let window = &windows[0];
  assert!(window.contains(timestamp));
}

#[test]
fn test_session_window_assigner_default_trigger() {
  let gap = Duration::from_secs(5);
  let assigner = SessionWindowAssigner::new(gap);
  let _trigger = assigner.default_trigger();
}

#[test]
fn test_session_window_assigner_is_event_time() {
  let gap = Duration::from_secs(5);
  let assigner = SessionWindowAssigner::new(gap);
  assert!(assigner.is_event_time());
}

#[test]
fn test_session_window_assigner_clone() {
  let gap = Duration::from_secs(5);
  let assigner1 = SessionWindowAssigner::new(gap);
  let assigner2 = assigner1.clone();
  assert_eq!(assigner1.gap(), assigner2.gap());
}

// ============================================================================
// CountWindowAssigner Tests
// ============================================================================

#[test]
fn test_count_window_assigner_new() {
  let count = 10;
  let assigner = CountWindowAssigner::new(count);
  assert_eq!(assigner.count(), count);
}

#[test]
fn test_count_window_assigner_assign_windows() {
  let count = 3;
  let assigner = CountWindowAssigner::new(count);
  let timestamp = Utc::now();

  // First window
  let windows1 = assigner.assign_windows(timestamp);
  assert_eq!(windows1.len(), 1);
  assert_eq!(windows1[0].id(), 0);

  // Second window (same timestamp, different call)
  let windows2 = assigner.assign_windows(timestamp);
  assert_eq!(windows2.len(), 1);
  // ID should increment when we reach the count
}

#[test]
fn test_count_window_assigner_default_trigger() {
  let count = 10;
  let assigner = CountWindowAssigner::new(count);
  let _trigger = assigner.default_trigger();
}

#[test]
fn test_count_window_assigner_is_event_time() {
  let count = 10;
  let assigner = CountWindowAssigner::new(count);
  assert!(!assigner.is_event_time());
}

#[test]
fn test_count_window_assigner_clone() {
  let count = 10;
  let assigner1 = CountWindowAssigner::new(count);
  let assigner2 = assigner1.clone();
  assert_eq!(assigner1.count(), assigner2.count());
}

// ============================================================================
// GlobalWindowAssigner Tests
// ============================================================================

#[test]
fn test_global_window_assigner_new() {
  let _assigner = GlobalWindowAssigner::new();
  // Just verify it can be created
}

#[test]
fn test_global_window_assigner_default() {
  let _assigner = GlobalWindowAssigner;
  // Just verify it can be created
}

#[test]
fn test_global_window_assigner_assign_windows() {
  let assigner = GlobalWindowAssigner::new();
  let timestamp = Utc::now();

  let windows = assigner.assign_windows(timestamp);
  assert_eq!(windows.len(), 1);
  assert_eq!(windows[0], GlobalWindow);
}

#[test]
fn test_global_window_assigner_default_trigger() {
  let assigner = GlobalWindowAssigner::new();
  let _trigger = assigner.default_trigger();
}

#[test]
fn test_global_window_assigner_is_event_time() {
  let assigner = GlobalWindowAssigner::new();
  assert!(assigner.is_event_time());
}

#[test]
fn test_global_window_display() {
  let window = GlobalWindow;
  assert_eq!(format!("{}", window), "GlobalWindow");
}

#[test]
fn test_global_window_trait() {
  let window = GlobalWindow;
  assert_eq!(window.end_time(), None);
  assert_eq!(window.start_time(), None);
  assert!(window.contains(Utc::now()));
}

#[test]
fn test_global_window_eq() {
  assert_eq!(GlobalWindow, GlobalWindow);
}

#[test]
fn test_global_window_hash() {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher1 = DefaultHasher::new();
  let mut hasher2 = DefaultHasher::new();
  GlobalWindow.hash(&mut hasher1);
  GlobalWindow.hash(&mut hasher2);
  assert_eq!(hasher1.finish(), hasher2.finish());
}

#[test]
fn test_global_window_clone() {
  let window1 = GlobalWindow;
  let window2 = window1.clone();
  assert_eq!(window1, window2);
}

// ============================================================================
// NeverTrigger Tests
// ============================================================================

#[test]
fn test_never_trigger_default() {
  let _trigger = NeverTrigger;
  // Just verify it can be created
}

#[test]
fn test_never_trigger_on_element() {
  let mut trigger = NeverTrigger;
  let timestamp = Utc::now();
  let window = GlobalWindow;

  let result = trigger.on_element(timestamp, &window);
  assert_eq!(result, TriggerResult::Continue);
}

#[test]
fn test_never_trigger_on_processing_time() {
  let mut trigger = NeverTrigger;
  let time = Utc::now();
  let window = GlobalWindow;

  let result = trigger.on_processing_time(time, &window);
  assert_eq!(result, TriggerResult::Continue);
}

#[test]
fn test_never_trigger_on_event_time() {
  let mut trigger = NeverTrigger;
  let watermark = Utc::now();
  let window = GlobalWindow;

  let result = trigger.on_event_time(watermark, &window);
  assert_eq!(result, TriggerResult::Continue);
}

#[test]
fn test_never_trigger_clear() {
  let mut trigger = NeverTrigger;
  let window = GlobalWindow;

  // Should not panic
  trigger.clear(&window);
}

#[test]
fn test_never_trigger_clone() {
  let mut trigger = NeverTrigger;
  let mut cloned = trigger.clone_trigger();
  let timestamp = Utc::now();
  let window = GlobalWindow;

  assert_eq!(
    trigger.on_element(timestamp, &window),
    cloned.on_element(timestamp, &window)
  );
}

// ============================================================================
// WindowConfig Tests
// ============================================================================

#[test]
fn test_window_config_default() {
  let config = WindowConfig::default();
  assert_eq!(config.late_data_policy, LateDataPolicy::Drop);
  assert_eq!(config.allowed_lateness, Duration::ZERO);
}

#[test]
fn test_window_config_new() {
  let config = WindowConfig::new();
  assert_eq!(config.late_data_policy, LateDataPolicy::Drop);
  assert_eq!(config.allowed_lateness, Duration::ZERO);
}

#[test]
fn test_window_config_with_late_data_policy() {
  let policy = LateDataPolicy::SideOutput;
  let config = WindowConfig::new().with_late_data_policy(policy);
  assert_eq!(config.late_data_policy, policy);
}

#[test]
fn test_window_config_with_allowed_lateness() {
  let lateness = Duration::from_secs(10);
  let config = WindowConfig::new().with_allowed_lateness(lateness);
  assert_eq!(config.allowed_lateness, lateness);
}

#[test]
fn test_window_config_clone() {
  let config1 = WindowConfig::new()
    .with_late_data_policy(LateDataPolicy::SideOutput)
    .with_allowed_lateness(Duration::from_secs(5));
  let config2 = config1.clone();
  assert_eq!(config1.late_data_policy, config2.late_data_policy);
  assert_eq!(config1.allowed_lateness, config2.allowed_lateness);
}

// ============================================================================
// Watermark Tests
// ============================================================================

#[test]
fn test_watermark_new() {
  let timestamp = Utc::now();
  let watermark = Watermark::new(timestamp);
  assert_eq!(watermark.timestamp, timestamp);
}

#[test]
fn test_watermark_min() {
  let watermark = Watermark::min();
  assert_eq!(watermark.timestamp, DateTime::<Utc>::MIN_UTC);
}

#[test]
fn test_watermark_max() {
  let watermark = Watermark::max();
  assert_eq!(watermark.timestamp, DateTime::<Utc>::MAX_UTC);
}

#[test]
fn test_watermark_is_end_of_stream() {
  let watermark = Watermark::max();
  assert!(watermark.is_end_of_stream());

  let watermark = Watermark::new(Utc::now());
  assert!(!watermark.is_end_of_stream());
}

#[test]
fn test_watermark_advance() {
  let mut watermark = Watermark::min();
  let timestamp1 = Utc::now();
  let timestamp2 = timestamp1 + chrono::Duration::seconds(10);

  assert!(watermark.advance(timestamp1));
  assert_eq!(watermark.timestamp, timestamp1);

  assert!(watermark.advance(timestamp2));
  assert_eq!(watermark.timestamp, timestamp2);

  // Advancing to earlier timestamp should not change watermark
  assert!(!watermark.advance(timestamp1));
  assert_eq!(watermark.timestamp, timestamp2);
}

#[test]
fn test_watermark_display() {
  let watermark = Watermark::max();
  let display = format!("{}", watermark);
  assert!(display.contains("END"));

  let timestamp = Utc::now();
  let watermark = Watermark::new(timestamp);
  let display = format!("{}", watermark);
  assert!(display.contains("Watermark"));
}

#[test]
fn test_watermark_eq() {
  let timestamp = Utc::now();
  let watermark1 = Watermark::new(timestamp);
  let watermark2 = Watermark::new(timestamp);
  assert_eq!(watermark1, watermark2);
}

#[test]
fn test_watermark_hash() {
  let timestamp = Utc::now();
  let watermark1 = Watermark::new(timestamp);
  let watermark2 = Watermark::new(timestamp);
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher1 = DefaultHasher::new();
  let mut hasher2 = DefaultHasher::new();
  watermark1.hash(&mut hasher1);
  watermark2.hash(&mut hasher2);
  assert_eq!(hasher1.finish(), hasher2.finish());
}

#[test]
fn test_watermark_ord() {
  let timestamp1 = Utc::now();
  let timestamp2 = timestamp1 + chrono::Duration::seconds(10);
  let watermark1 = Watermark::new(timestamp1);
  let watermark2 = Watermark::new(timestamp2);

  assert!(watermark1 < watermark2);
}

#[test]
fn test_watermark_clone() {
  let timestamp = Utc::now();
  let watermark1 = Watermark::new(timestamp);
  let watermark2 = watermark1.clone();
  assert_eq!(watermark1, watermark2);
}

// ============================================================================
// MonotonicWatermarkGenerator Tests
// ============================================================================

#[test]
fn test_monotonic_watermark_generator_new() {
  let generator = MonotonicWatermarkGenerator::new();
  assert_eq!(
    generator.current_watermark().timestamp,
    DateTime::<Utc>::MIN_UTC
  );
}

#[test]
fn test_monotonic_watermark_generator_default() {
  let generator = MonotonicWatermarkGenerator::default();
  assert_eq!(
    generator.current_watermark().timestamp,
    DateTime::<Utc>::MIN_UTC
  );
}

#[test]
fn test_monotonic_watermark_generator_on_event() {
  let mut generator = MonotonicWatermarkGenerator::new();
  let timestamp1 = Utc::now();
  let timestamp2 = timestamp1 + chrono::Duration::seconds(10);

  let result1 = generator.on_event(timestamp1);
  assert!(result1.is_some());
  assert_eq!(result1.unwrap().timestamp, timestamp1);

  let result2 = generator.on_event(timestamp2);
  assert!(result2.is_some());
  assert_eq!(result2.unwrap().timestamp, timestamp2);

  // Older timestamp should not emit new watermark
  let result3 = generator.on_event(timestamp1);
  assert!(result3.is_none());
}

#[test]
fn test_monotonic_watermark_generator_on_periodic_emit() {
  let mut generator = MonotonicWatermarkGenerator::new();
  let result = generator.on_periodic_emit();
  assert!(result.is_none());
}

#[test]
fn test_monotonic_watermark_generator_current_watermark() {
  let mut generator = MonotonicWatermarkGenerator::new();
  let timestamp = Utc::now();
  generator.on_event(timestamp);

  let current = generator.current_watermark();
  assert_eq!(current.timestamp, timestamp);
}

#[test]
fn test_monotonic_watermark_generator_on_end_of_stream() {
  let mut generator = MonotonicWatermarkGenerator::new();
  let final_watermark = generator.on_end_of_stream();
  assert!(final_watermark.is_end_of_stream());
}

#[test]
fn test_monotonic_watermark_generator_clone() {
  let mut generator1 = MonotonicWatermarkGenerator::new();
  let timestamp = Utc::now();
  generator1.on_event(timestamp);

  let generator2 = generator1.clone();
  assert_eq!(
    generator1.current_watermark(),
    generator2.current_watermark()
  );
}

// ============================================================================
// BoundedOutOfOrdernessGenerator Tests
// ============================================================================

#[test]
fn test_bounded_out_of_orderness_generator_new() {
  let max_out_of_orderness = Duration::from_secs(5);
  let generator = BoundedOutOfOrdernessGenerator::new(max_out_of_orderness);
  assert_eq!(
    generator.current_watermark().timestamp,
    DateTime::<Utc>::MIN_UTC
  );
}

#[test]
fn test_bounded_out_of_orderness_generator_on_event() {
  let max_out_of_orderness = Duration::from_secs(5);
  let mut generator = BoundedOutOfOrdernessGenerator::new(max_out_of_orderness);
  let timestamp1 = Utc::now();
  let timestamp2 = timestamp1 + chrono::Duration::seconds(10);

  let result1 = generator.on_event(timestamp1);
  assert!(result1.is_some());

  let result2 = generator.on_event(timestamp2);
  assert!(result2.is_some());
  // Watermark should be timestamp - max_out_of_orderness
  let expected_watermark = timestamp2 - chrono::Duration::from_std(max_out_of_orderness).unwrap();
  assert_eq!(result2.unwrap().timestamp, expected_watermark);
}

#[test]
fn test_bounded_out_of_orderness_generator_on_periodic_emit() {
  let max_out_of_orderness = Duration::from_secs(5);
  let mut generator = BoundedOutOfOrdernessGenerator::new(max_out_of_orderness);
  let timestamp = Utc::now();
  generator.on_event(timestamp);

  let result = generator.on_periodic_emit();
  assert!(result.is_some());
}

#[test]
fn test_bounded_out_of_orderness_generator_current_watermark() {
  let max_out_of_orderness = Duration::from_secs(5);
  let mut generator = BoundedOutOfOrdernessGenerator::new(max_out_of_orderness);
  let timestamp = Utc::now();
  generator.on_event(timestamp);

  let current = generator.current_watermark();
  let expected = timestamp - chrono::Duration::from_std(max_out_of_orderness).unwrap();
  assert_eq!(current.timestamp, expected);
}

#[test]
fn test_bounded_out_of_orderness_generator_on_end_of_stream() {
  let max_out_of_orderness = Duration::from_secs(5);
  let mut generator = BoundedOutOfOrdernessGenerator::new(max_out_of_orderness);
  let final_watermark = generator.on_end_of_stream();
  assert!(final_watermark.is_end_of_stream());
}

#[test]
fn test_bounded_out_of_orderness_generator_clone() {
  let max_out_of_orderness = Duration::from_secs(5);
  let mut generator1 = BoundedOutOfOrdernessGenerator::new(max_out_of_orderness);
  let timestamp = Utc::now();
  generator1.on_event(timestamp);

  let generator2 = generator1.clone();
  assert_eq!(
    generator1.current_watermark(),
    generator2.current_watermark()
  );
}

// ============================================================================
// PeriodicWatermarkGenerator Tests
// ============================================================================

#[test]
fn test_periodic_watermark_generator_new() {
  let max_out_of_orderness = Duration::from_secs(5);
  let interval = Duration::from_secs(10);
  let _generator = PeriodicWatermarkGenerator::new(max_out_of_orderness, interval);
}

#[test]
fn test_periodic_watermark_generator_on_event() {
  let max_out_of_orderness = Duration::from_secs(5);
  let interval = Duration::from_secs(10);
  let mut generator = PeriodicWatermarkGenerator::new(max_out_of_orderness, interval);
  let timestamp = Utc::now();

  // on_event should not emit watermark
  let result = generator.on_event(timestamp);
  assert!(result.is_none());
}

#[test]
fn test_periodic_watermark_generator_on_periodic_emit() {
  let max_out_of_orderness = Duration::from_secs(5);
  let interval = Duration::from_secs(10);
  let mut generator = PeriodicWatermarkGenerator::new(max_out_of_orderness, interval);
  let timestamp = Utc::now();
  generator.on_event(timestamp);

  // First periodic emit should work
  let result1 = generator.on_periodic_emit();
  assert!(result1.is_some());

  // Immediate second call might not emit (within interval)
  // But we can't control the timing exactly, so just check it doesn't panic
  let _result2 = generator.on_periodic_emit();
}

#[test]
fn test_periodic_watermark_generator_current_watermark() {
  let max_out_of_orderness = Duration::from_secs(5);
  let interval = Duration::from_secs(10);
  let mut generator = PeriodicWatermarkGenerator::new(max_out_of_orderness, interval);
  let timestamp = Utc::now();
  generator.on_event(timestamp);

  let current = generator.current_watermark();
  let expected = timestamp - chrono::Duration::from_std(max_out_of_orderness).unwrap();
  assert_eq!(current.timestamp, expected);
}

#[test]
fn test_periodic_watermark_generator_on_end_of_stream() {
  let max_out_of_orderness = Duration::from_secs(5);
  let interval = Duration::from_secs(10);
  let mut generator = PeriodicWatermarkGenerator::new(max_out_of_orderness, interval);
  let final_watermark = generator.on_end_of_stream();
  assert!(final_watermark.is_end_of_stream());
}

#[test]
fn test_periodic_watermark_generator_clone() {
  let max_out_of_orderness = Duration::from_secs(5);
  let interval = Duration::from_secs(10);
  let mut generator1 = PeriodicWatermarkGenerator::new(max_out_of_orderness, interval);
  let timestamp = Utc::now();
  generator1.on_event(timestamp);

  let generator2 = generator1.clone();
  assert_eq!(
    generator1.current_watermark(),
    generator2.current_watermark()
  );
}

// ============================================================================
// LateDataResult Tests
// ============================================================================

#[test]
fn test_late_data_result_should_process() {
  assert!(LateDataResult::OnTime(42).should_process());
  assert!(LateDataResult::WithinLateness(42).should_process());
  assert!(!LateDataResult::<i32>::Drop.should_process());
  assert!(!LateDataResult::SideOutput(42).should_process());
}

#[test]
fn test_late_data_result_into_processable() {
  assert_eq!(LateDataResult::OnTime(42).into_processable(), Some(42));
  assert_eq!(
    LateDataResult::WithinLateness(42).into_processable(),
    Some(42)
  );
  assert_eq!(LateDataResult::<i32>::Drop.into_processable(), None);
  assert_eq!(LateDataResult::SideOutput(42).into_processable(), None);
}

#[test]
fn test_late_data_result_is_side_output() {
  assert!(!LateDataResult::OnTime(42).is_side_output());
  assert!(!LateDataResult::WithinLateness(42).is_side_output());
  assert!(!LateDataResult::<i32>::Drop.is_side_output());
  assert!(LateDataResult::SideOutput(42).is_side_output());
}

#[test]
fn test_late_data_result_into_side_output() {
  assert_eq!(LateDataResult::OnTime(42).into_side_output(), None);
  assert_eq!(LateDataResult::WithinLateness(42).into_side_output(), None);
  assert_eq!(LateDataResult::<i32>::Drop.into_side_output(), None);
  assert_eq!(LateDataResult::SideOutput(42).into_side_output(), Some(42));
}

// ============================================================================
// LateDataHandler Tests
// ============================================================================

#[test]
fn test_late_data_handler_new() {
  let policy = LateDataPolicy::Drop;
  let handler = LateDataHandler::new(policy);
  assert_eq!(handler.policy(), &policy);
}

#[test]
fn test_late_data_handler_drop_late() {
  let handler = LateDataHandler::drop_late();
  assert_eq!(handler.policy(), &LateDataPolicy::Drop);
}

#[test]
fn test_late_data_handler_with_allowed_lateness() {
  let lateness = Duration::from_secs(10);
  let handler = LateDataHandler::with_allowed_lateness(lateness);
  assert_eq!(handler.get_allowed_lateness(), lateness);
}

#[test]
fn test_late_data_handler_redirect_to_side_output() {
  let handler = LateDataHandler::redirect_to_side_output();
  assert_eq!(handler.policy(), &LateDataPolicy::SideOutput);
}

#[test]
fn test_late_data_handler_evaluate_on_time() {
  let mut handler = LateDataHandler::new(LateDataPolicy::Drop);
  let element_timestamp = Utc::now();
  let watermark = Watermark::new(element_timestamp);
  let element = 42;

  let result = handler.evaluate(element_timestamp, &watermark, element);
  assert!(matches!(result, LateDataResult::OnTime(42)));
}

#[test]
fn test_late_data_handler_evaluate_late_within_lateness() {
  let lateness = Duration::from_secs(10);
  let mut handler = LateDataHandler::with_allowed_lateness(lateness);
  let watermark_timestamp = Utc::now();
  let watermark = Watermark::new(watermark_timestamp);
  let element_timestamp = watermark_timestamp - chrono::Duration::from_std(lateness).unwrap()
    + chrono::Duration::seconds(1);
  let element = 42;

  let result = handler.evaluate(element_timestamp, &watermark, element);
  assert!(matches!(result, LateDataResult::WithinLateness(42)));
}

#[test]
fn test_late_data_handler_evaluate_late_drop() {
  let mut handler = LateDataHandler::drop_late();
  let watermark_timestamp = Utc::now();
  let watermark = Watermark::new(watermark_timestamp);
  let element_timestamp = watermark_timestamp - chrono::Duration::seconds(100);
  let element = 42;

  let result = handler.evaluate(element_timestamp, &watermark, element);
  assert!(matches!(result, LateDataResult::Drop));
}

#[test]
fn test_late_data_handler_evaluate_late_side_output() {
  let mut handler = LateDataHandler::redirect_to_side_output();
  let watermark_timestamp = Utc::now();
  let watermark = Watermark::new(watermark_timestamp);
  let element_timestamp = watermark_timestamp - chrono::Duration::seconds(100);
  let element = 42;

  let result = handler.evaluate(element_timestamp, &watermark, element);
  assert!(matches!(result, LateDataResult::SideOutput(42)));
}

#[test]
fn test_late_data_handler_stats() {
  let mut handler = LateDataHandler::drop_late();
  let watermark = Watermark::new(Utc::now());
  let late_timestamp = watermark.timestamp - chrono::Duration::seconds(100);

  handler.evaluate(watermark.timestamp, &watermark, 1);
  handler.evaluate(late_timestamp, &watermark, 2);

  let stats = handler.stats();
  assert_eq!(stats.on_time, 1);
  assert!(stats.dropped > 0);
}

#[test]
fn test_late_data_handler_reset_stats() {
  let mut handler = LateDataHandler::drop_late();
  let watermark = Watermark::new(Utc::now());
  let late_timestamp = watermark.timestamp - chrono::Duration::seconds(100);

  handler.evaluate(watermark.timestamp, &watermark, 1);
  handler.evaluate(late_timestamp, &watermark, 2);

  handler.reset_stats();
  let stats = handler.stats();
  assert_eq!(stats.on_time, 0);
  assert_eq!(stats.dropped, 0);
}

#[test]
fn test_late_data_handler_get_allowed_lateness() {
  let lateness = Duration::from_secs(5);
  let handler = LateDataHandler::with_allowed_lateness(lateness);
  assert_eq!(handler.get_allowed_lateness(), lateness);

  let handler = LateDataHandler::drop_late();
  assert_eq!(handler.get_allowed_lateness(), Duration::ZERO);
}

#[test]
fn test_late_data_handler_clone() {
  let handler1 = LateDataHandler::with_allowed_lateness(Duration::from_secs(10));
  let handler2 = handler1.clone();
  assert_eq!(handler1.policy(), handler2.policy());
  assert_eq!(
    handler1.get_allowed_lateness(),
    handler2.get_allowed_lateness()
  );
}

// ============================================================================
// LateDataStats Tests
// ============================================================================

#[test]
fn test_late_data_stats_default() {
  let stats = LateDataStats::default();
  assert_eq!(stats.on_time, 0);
  assert_eq!(stats.within_lateness, 0);
  assert_eq!(stats.dropped, 0);
  assert_eq!(stats.side_output, 0);
}

#[test]
fn test_late_data_stats_clone() {
  let stats1 = LateDataStats {
    on_time: 10,
    dropped: 5,
    ..Default::default()
  };
  let stats2 = stats1.clone();
  assert_eq!(stats1.on_time, stats2.on_time);
  assert_eq!(stats1.dropped, stats2.dropped);
}

// ============================================================================
// WindowState Tests
// ============================================================================

#[test]
fn test_window_state_new() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);
  let state: WindowState<TimeWindow, i32> = WindowState::new(window.clone());

  assert_eq!(state.window, window);
  assert!(state.elements.is_empty());
  assert_eq!(state.count, 0);
  assert!(!state.triggered);
  assert!(state.last_watermark.is_none());
}

#[test]
fn test_window_state_add() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);
  let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

  let timestamp = Utc::now();
  state.add(timestamp, 42);

  assert_eq!(state.elements.len(), 1);
  assert_eq!(state.count, 1);
  assert_eq!(state.elements[0], (timestamp, 42));
}

#[test]
fn test_window_state_update_watermark() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);
  let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

  let watermark = Watermark::new(Utc::now());
  state.update_watermark(watermark.clone());

  assert_eq!(state.last_watermark, Some(watermark));
}

#[test]
fn test_window_state_can_be_gc() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);
  let mut state: WindowState<TimeWindow, i32> = WindowState::new(window.clone());

  // Not triggered yet
  let watermark = Watermark::new(end);
  assert!(!state.can_be_gc(&watermark, chrono::Duration::zero()));

  // Triggered and watermark passed
  state.mark_triggered();
  let watermark = Watermark::new(end + chrono::Duration::seconds(1));
  assert!(state.can_be_gc(&watermark, chrono::Duration::zero()));

  // With allowed lateness
  let watermark = Watermark::new(end + chrono::Duration::seconds(5));
  let allowed_lateness = chrono::Duration::seconds(10);
  assert!(!state.can_be_gc(&watermark, allowed_lateness));
}

#[test]
fn test_window_state_can_be_gc_global_window() {
  let window = GlobalWindow;
  let mut state: WindowState<GlobalWindow, i32> = WindowState::new(window);
  state.mark_triggered();
  let watermark = Watermark::new(Utc::now());

  // Global windows are never GC'd
  assert!(!state.can_be_gc(&watermark, chrono::Duration::zero()));
}

#[test]
fn test_window_state_clear() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);
  let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

  state.add(Utc::now(), 42);
  state.add(Utc::now(), 43);
  assert_eq!(state.count, 2);

  state.clear();
  assert!(state.elements.is_empty());
  assert_eq!(state.count, 0);
}

#[test]
fn test_window_state_mark_triggered() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);
  let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

  assert!(!state.triggered);
  state.mark_triggered();
  assert!(state.triggered);
}

#[test]
fn test_window_state_elements() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);
  let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

  let timestamp1 = Utc::now();
  let timestamp2 = timestamp1 + chrono::Duration::seconds(1);
  state.add(timestamp1, 42);
  state.add(timestamp2, 43);

  let elements = state.elements();
  assert_eq!(elements.len(), 2);
  assert_eq!(elements[0], (timestamp1, 42));
  assert_eq!(elements[1], (timestamp2, 43));
}

#[test]
fn test_window_state_values() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);
  let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

  state.add(Utc::now(), 42);
  state.add(Utc::now(), 43);
  state.add(Utc::now(), 44);

  let values = state.values();
  assert_eq!(values, vec![42, 43, 44]);
}

#[test]
fn test_window_state_clone() {
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);
  let mut state1: WindowState<TimeWindow, i32> = WindowState::new(window.clone());

  state1.add(Utc::now(), 42);
  state1.mark_triggered();

  let state2 = state1.clone();
  assert_eq!(state1.window, state2.window);
  assert_eq!(state1.elements, state2.elements);
  assert_eq!(state1.count, state2.count);
  assert_eq!(state1.triggered, state2.triggered);
}
