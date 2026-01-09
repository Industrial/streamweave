//! Tests for window module

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use streamweave::window::*;

#[test]
fn test_trigger_result_variants() {
  assert_eq!(TriggerResult::Continue, TriggerResult::Continue);
  assert_eq!(TriggerResult::Fire, TriggerResult::Fire);
  assert_eq!(TriggerResult::FireAndPurge, TriggerResult::FireAndPurge);
  assert_eq!(TriggerResult::Purge, TriggerResult::Purge);
}

#[test]
fn test_late_data_policy() {
  assert_eq!(LateDataPolicy::default(), LateDataPolicy::Drop);
  assert_ne!(LateDataPolicy::Drop, LateDataPolicy::SideOutput);

  match LateDataPolicy::AllowLateness(std::time::Duration::from_secs(5)) {
    LateDataPolicy::AllowLateness(d) => assert_eq!(d.as_secs(), 5),
    _ => assert!(false),
  }
}

#[test]
fn test_window_error_display() {
  let error = WindowError::InvalidConfig("test".to_string());
  let display = format!("{}", error);
  assert!(display.contains("Invalid window config"));

  let error2 = WindowError::NotFound("test".to_string());
  let display2 = format!("{}", error2);
  assert!(display2.contains("Window not found"));
}

#[test]
fn test_time_window_new() {
  let start = Utc::now();
  let end = start + ChronoDuration::seconds(10);
  let window = TimeWindow::new(start, end);

  assert_eq!(window.start(), start);
  assert_eq!(window.end(), end);
}

#[test]
fn test_time_window_duration() {
  let start = Utc::now();
  let end = start + ChronoDuration::seconds(10);
  let window = TimeWindow::new(start, end);

  let duration = window.duration();
  assert_eq!(duration.num_seconds(), 10);
}

#[test]
fn test_time_window_contains() {
  let start = Utc::now();
  let end = start + ChronoDuration::seconds(10);
  let window = TimeWindow::new(start, end);

  assert!(window.contains(start));
  assert!(window.contains(start + ChronoDuration::seconds(5)));
  assert!(!window.contains(end));
  assert!(!window.contains(start - ChronoDuration::seconds(1)));
}

#[test]
fn test_time_window_intersects() {
  let start1 = Utc::now();
  let end1 = start1 + ChronoDuration::seconds(10);
  let window1 = TimeWindow::new(start1, end1);

  let start2 = start1 + ChronoDuration::seconds(5);
  let end2 = start2 + ChronoDuration::seconds(10);
  let window2 = TimeWindow::new(start2, end2);

  assert!(window1.intersects(&window2));
  assert!(window2.intersects(&window1));
}

#[test]
fn test_time_window_merge() {
  let start1 = Utc::now();
  let end1 = start1 + ChronoDuration::seconds(10);
  let window1 = TimeWindow::new(start1, end1);

  let start2 = start1 + ChronoDuration::seconds(5);
  let end2 = start2 + ChronoDuration::seconds(10);
  let window2 = TimeWindow::new(start2, end2);

  let merged = window1.merge(&window2);
  assert!(merged.is_some());
  let merged_window = merged.unwrap();
  assert_eq!(merged_window.start(), start1);
  assert_eq!(merged_window.end(), end2);
}

#[test]
fn test_time_window_merge_non_overlapping() {
  let start1 = Utc::now();
  let end1 = start1 + ChronoDuration::seconds(10);
  let window1 = TimeWindow::new(start1, end1);

  let start2 = start1 + ChronoDuration::seconds(20);
  let end2 = start2 + ChronoDuration::seconds(10);
  let window2 = TimeWindow::new(start2, end2);

  let merged = window1.merge(&window2);
  assert!(merged.is_none());
}

#[test]
fn test_time_window_equality() {
  let start = Utc::now();
  let end = start + ChronoDuration::seconds(10);
  let window1 = TimeWindow::new(start, end);
  let window2 = TimeWindow::new(start, end);

  assert_eq!(window1, window2);
}

#[test]
fn test_time_window_ordering() {
  let start1 = Utc::now();
  let end1 = start1 + ChronoDuration::seconds(10);
  let window1 = TimeWindow::new(start1, end1);

  let start2 = start1 + ChronoDuration::seconds(5);
  let end2 = start2 + ChronoDuration::seconds(10);
  let window2 = TimeWindow::new(start2, end2);

  assert!(window1 < window2);
}

#[test]
fn test_time_window_display() {
  let start = Utc::now();
  let end = start + ChronoDuration::seconds(10);
  let window = TimeWindow::new(start, end);

  let display = format!("{}", window);
  assert!(display.contains("["));
  assert!(display.contains(")"));
}

#[test]
fn test_time_window_impl_window() {
  let start = Utc::now();
  let end = start + ChronoDuration::seconds(10);
  let window = TimeWindow::new(start, end);

  assert_eq!(window.start_time(), Some(start));
  assert_eq!(window.end_time(), Some(end));
  assert!(window.contains(start + ChronoDuration::seconds(5)));
}

#[test]
fn test_count_window_new() {
  let window = CountWindow::new(1, 100);

  assert_eq!(window.id(), 1);
  assert_eq!(window.max_count(), 100);
}

#[test]
fn test_count_window_equality() {
  let window1 = CountWindow::new(1, 100);
  let window2 = CountWindow::new(1, 100);
  let window3 = CountWindow::new(2, 100);

  assert_eq!(window1, window2);
  assert_ne!(window1, window3);
}
