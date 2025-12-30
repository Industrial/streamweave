//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.

use chrono::Utc;
use std::time::Duration;
use streamweave_window::{TimeWindow, TumblingWindowAssigner, WindowConfig};

/// Test: Tumbling Window
///
/// This test recreates the "Tumbling Window" example from README.md.
#[test]
fn test_tumbling_window_example() {
  // Example from README.md lines 33-45
  use std::time::Duration;
  use streamweave_window::{TumblingWindowAssigner, WindowConfig};

  // Create a tumbling window of 5 seconds
  let assigner = TumblingWindowAssigner::new(Duration::from_secs(5));
  let config = WindowConfig::default();

  // Verify assigner was created
  assert_eq!(assigner.size(), Duration::from_secs(5));
  assert_eq!(
    config.late_data_policy,
    streamweave_window::LateDataPolicy::Drop
  );
}

/// Test: Sliding Window
///
/// This test recreates the "Sliding Window" example from README.md.
#[test]
fn test_sliding_window_example() {
  // Example from README.md lines 47-57
  use streamweave_window::SlidingWindowAssigner;

  // Create a sliding window: 10 second size, 5 second slide
  let assigner = SlidingWindowAssigner::new(Duration::from_secs(10), Duration::from_secs(5));

  assert_eq!(assigner.size(), Duration::from_secs(10));
  assert_eq!(assigner.slide(), Duration::from_secs(5));
}

/// Test: Session Window
///
/// This test recreates the "Session Window" example from README.md.
#[test]
fn test_session_window_example() {
  // Example from README.md lines 140-151
  use streamweave_window::SessionWindowAssigner;

  // Session gap of 30 seconds
  let assigner = SessionWindowAssigner::new(Duration::from_secs(30));

  assert_eq!(assigner.gap(), Duration::from_secs(30));
}

/// Test: Count-Based Windows
///
/// This test recreates the "Count-Based Windows" example from README.md.
#[test]
fn test_count_based_windows_example() {
  // Example from README.md lines 153-162
  use streamweave_window::CountWindowAssigner;

  // Window of 100 elements
  let assigner = CountWindowAssigner::new(100);

  assert_eq!(assigner.count(), 100);
}

/// Test: Time-Based Windows
///
/// This test recreates the "Time-Based Windows" example from README.md.
#[test]
fn test_time_based_windows_example() {
  // Example from README.md lines 205-223
  use chrono::Utc;
  use streamweave_window::{Duration, TimeWindow};

  // Create a time window
  let start = Utc::now();
  let end = start + chrono::Duration::seconds(10);
  let window = TimeWindow::new(start, end);

  // Check if timestamp is in window
  let timestamp = Utc::now();
  assert!(window.contains(timestamp));
}

/// Test: Late Data Handling
///
/// This test recreates the "Late Data Handling" example from README.md.
#[test]
fn test_late_data_handling_example() {
  // Example from README.md lines 178-203
  use std::time::Duration;
  use streamweave_window::{LateDataPolicy, WindowConfig};

  // Allow lateness up to 1 minute
  let config = WindowConfig::default()
    .with_late_data_policy(LateDataPolicy::AllowLateness(Duration::from_secs(60)))
    .with_allowed_lateness(Duration::from_secs(60));

  assert!(matches!(
    config.late_data_policy,
    LateDataPolicy::AllowLateness(_)
  ));
  assert_eq!(config.allowed_lateness, Duration::from_secs(60));
}
