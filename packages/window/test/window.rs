//! Property-based tests for window operations using proptest.

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use proptest::prelude::*;
use std::time::Duration;
use streamweave_window::*;

/// Helper to generate valid DateTime values within a reasonable range.
fn datetime_strategy() -> impl Strategy<Value = DateTime<Utc>> {
  // Generate timestamps between 2000-01-01 and 2100-01-01
  (946_684_800i64..4_102_444_800i64)
    .prop_map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_else(|| Utc::now()))
}

/// Helper to generate positive durations.
fn duration_strategy() -> impl Strategy<Value = Duration> {
  (1u64..=86_400_000).prop_map(|ms| Duration::from_millis(ms)) // 1ms to 1 day
}

/// Helper to generate chrono durations.
fn chrono_duration_strategy() -> impl Strategy<Value = ChronoDuration> {
  (1i64..=86_400_000).prop_map(|ms| ChronoDuration::milliseconds(ms))
}

proptest! {
  // ==========================================================================
  // TimeWindow Property Tests
  // ==========================================================================

  #[test]
  fn prop_time_window_contains_itself(
    start in datetime_strategy(),
    duration in duration_strategy()
  ) {
    let end = start + ChronoDuration::from_std(duration).unwrap_or(ChronoDuration::zero());
    if end > start {
      let window = TimeWindow::new(start, end);

      // Start time should be contained (inclusive)
      prop_assert!(window.contains(start));

      // Times just before start should not be contained
      if let Some(before_start) = start.checked_sub_signed(ChronoDuration::milliseconds(1)) {
        prop_assert!(!window.contains(before_start));
      }

      // End time should not be contained (exclusive)
      prop_assert!(!window.contains(end));
    }
  }

  #[test]
  fn prop_time_window_intersects_self(
    start in datetime_strategy(),
    duration in duration_strategy()
  ) {
    let end = start + ChronoDuration::from_std(duration).unwrap_or(ChronoDuration::zero());
    if end > start {
      let window = TimeWindow::new(start, end);
      // A window should intersect itself
      prop_assert!(window.intersects(&window));
    }
  }

  #[test]
  fn prop_time_window_merge_commutative(
    start1 in datetime_strategy(),
    duration1 in duration_strategy(),
    start2 in datetime_strategy(),
    duration2 in duration_strategy()
  ) {
    let end1 = start1 + ChronoDuration::from_std(duration1).unwrap_or(ChronoDuration::zero());
    let end2 = start2 + ChronoDuration::from_std(duration2).unwrap_or(ChronoDuration::zero());

    if end1 > start1 && end2 > start2 {
      let w1 = TimeWindow::new(start1, end1);
      let w2 = TimeWindow::new(start2, end2);

      let merged1 = w1.merge(&w2);
      let merged2 = w2.merge(&w1);

      // Merge should be commutative (if both succeed, results should be equal)
      match (merged1, merged2) {
        (Some(m1), Some(m2)) => {
          prop_assert_eq!(m1.start(), m2.start());
          prop_assert_eq!(m1.end(), m2.end());
        }
        (None, None) => {
          // Both failed - that's fine
        }
        _ => {
          prop_assert!(false, "Merge should be commutative");
        }
      }
    }
  }

  #[test]
  fn prop_time_window_merge_idempotent(
    start in datetime_strategy(),
    duration in duration_strategy()
  ) {
    let end = start + ChronoDuration::from_std(duration).unwrap_or(ChronoDuration::zero());
    if end > start {
      let window = TimeWindow::new(start, end);

      // Merging a window with itself should return the same window
      if let Some(merged) = window.merge(&window) {
        prop_assert_eq!(merged.start(), window.start());
        prop_assert_eq!(merged.end(), window.end());
      }
    }
  }

  #[test]
  fn prop_time_window_merge_contains_original(
    start1 in datetime_strategy(),
    duration1 in duration_strategy(),
    start2 in datetime_strategy(),
    duration2 in duration_strategy()
  ) {
    let end1 = start1 + ChronoDuration::from_std(duration1).unwrap_or(ChronoDuration::zero());
    let end2 = start2 + ChronoDuration::from_std(duration2).unwrap_or(ChronoDuration::zero());

    if end1 > start1 && end2 > start2 {
      let w1 = TimeWindow::new(start1, end1);
      let w2 = TimeWindow::new(start2, end2);

      if let Some(merged) = w1.merge(&w2) {
        // Merged window should contain all timestamps from both original windows
        let test_times = vec![w1.start(), w1.end() - ChronoDuration::milliseconds(1),
                              w2.start(), w2.end() - ChronoDuration::milliseconds(1)];

        for ts in test_times {
          if w1.contains(ts) || w2.contains(ts) {
            prop_assert!(merged.contains(ts) || ts == merged.end());
          }
        }

        // Merged window should start at or before both windows
        prop_assert!(merged.start() <= w1.start());
        prop_assert!(merged.start() <= w2.start());

        // Merged window should end at or after both windows
        prop_assert!(merged.end() >= w1.end());
        prop_assert!(merged.end() >= w2.end());
      }
    }
  }

  #[test]
  fn prop_time_window_ordering_consistent(
    start1 in datetime_strategy(),
    duration1 in duration_strategy(),
    start2 in datetime_strategy(),
    duration2 in duration_strategy()
  ) {
    let end1 = start1 + ChronoDuration::from_std(duration1).unwrap_or(ChronoDuration::zero());
    let end2 = start2 + ChronoDuration::from_std(duration2).unwrap_or(ChronoDuration::zero());

    if end1 > start1 && end2 > start2 {
      let w1 = TimeWindow::new(start1, end1);
      let w2 = TimeWindow::new(start2, end2);

      // Ordering should be consistent with start time, then end time
      match w1.start().cmp(&w2.start()) {
        std::cmp::Ordering::Less => prop_assert!(w1 < w2),
        std::cmp::Ordering::Greater => prop_assert!(w1 > w2),
        std::cmp::Ordering::Equal => {
          match w1.end().cmp(&w2.end()) {
            std::cmp::Ordering::Less => prop_assert!(w1 < w2),
            std::cmp::Ordering::Greater => prop_assert!(w1 > w2),
            std::cmp::Ordering::Equal => prop_assert_eq!(w1, w2),
          }
        }
      }
    }
  }

  #[test]
  fn prop_time_window_duration_positive(
    start in datetime_strategy(),
    duration in duration_strategy()
  ) {
    let end = start + ChronoDuration::from_std(duration).unwrap_or(ChronoDuration::zero());
    if end > start {
      let window = TimeWindow::new(start, end);
      let duration = window.duration();
      prop_assert!(duration.num_milliseconds() >= 0);
    }
  }

  // ==========================================================================
  // SessionWindow Property Tests
  // ==========================================================================

  #[test]
  fn prop_session_window_contains_its_start(
    start in datetime_strategy(),
    gap in duration_strategy()
  ) {
    let session = SessionWindow::from_element(start, gap);
    prop_assert!(session.contains(start));
  }

  #[test]
  fn prop_session_window_extend_always_contains_original(
    start in datetime_strategy(),
    gap in duration_strategy(),
    extend_time in datetime_strategy()
  ) {
    let mut session = SessionWindow::from_element(start, gap);
    let original_start = session.start();
    let original_end = session.end();

    session.extend(extend_time);

    // Extended session should still contain the original start
    prop_assert!(session.contains(original_start));

    // Extended session should contain the extend time
    prop_assert!(session.contains(extend_time));

    // Extended session should start at or before original start
    prop_assert!(session.start() <= original_start);

    // Extended session should end at or after original end
    prop_assert!(session.end() >= original_end);
  }

  #[test]
  fn prop_session_window_merge_commutative(
    start1 in datetime_strategy(),
    gap1 in duration_strategy(),
    start2 in datetime_strategy(),
    gap2 in duration_strategy()
  ) {
    let s1 = SessionWindow::from_element(start1, gap1);
    let s2 = SessionWindow::from_element(start2, gap2);

    let merged1 = s1.merge(&s2);
    let merged2 = s2.merge(&s1);

    match (merged1, merged2) {
      (Some(m1), Some(m2)) => {
        prop_assert_eq!(m1.start(), m2.start());
        prop_assert_eq!(m1.end(), m2.end());
      }
      (None, None) => {
        // Both failed - that's fine
      }
      _ => {
        prop_assert!(false, "Merge should be commutative");
      }
    }
  }

  #[test]
  fn prop_session_window_should_merge_symmetric(
    start1 in datetime_strategy(),
    gap1 in duration_strategy(),
    start2 in datetime_strategy(),
    gap2 in duration_strategy()
  ) {
    let s1 = SessionWindow::from_element(start1, gap1);
    let s2 = SessionWindow::from_element(start2, gap2);

    // should_merge should be symmetric
    prop_assert_eq!(s1.should_merge(&s2), s2.should_merge(&s1));
  }

  // ==========================================================================
  // Window Assigner Property Tests
  // ==========================================================================

  #[test]
  fn prop_tumbling_assigner_assigns_single_window(
    timestamp in datetime_strategy(),
    size in duration_strategy()
  ) {
    if size.as_millis() > 0 {
      let assigner = TumblingWindowAssigner::new(size);
      let windows = assigner.assign_windows(timestamp);

      // Tumbling assigner should always assign exactly one window
      prop_assert_eq!(windows.len(), 1);

      // The assigned window should contain the timestamp
      prop_assert!(windows[0].contains(timestamp));
    }
  }

  #[test]
  fn prop_tumbling_assigner_windows_non_overlapping(
    timestamp1 in datetime_strategy(),
    timestamp2 in datetime_strategy(),
    size in duration_strategy()
  ) {
    if size.as_millis() > 0 {
      let assigner = TumblingWindowAssigner::new(size);
      let windows1 = assigner.assign_windows(timestamp1);
      let windows2 = assigner.assign_windows(timestamp2);

      // If timestamps are far enough apart, windows should not overlap
      let time_diff = (timestamp1 - timestamp2).num_milliseconds().abs();
      let size_ms = size.as_millis() as i64;

      if time_diff >= size_ms * 2 {
        prop_assert!(!windows1[0].intersects(&windows2[0]));
      }
    }
  }

  #[test]
  fn prop_sliding_assigner_assigns_multiple_windows(
    timestamp in datetime_strategy(),
    size in duration_strategy(),
    slide in duration_strategy()
  ) {
    if size.as_millis() > 0 && slide.as_millis() > 0 && size >= slide {
      let assigner = SlidingWindowAssigner::new(size, slide);
      let windows = assigner.assign_windows(timestamp);

      // Sliding assigner should assign multiple windows when size > slide
      if size > slide {
        prop_assert!(windows.len() > 1);
      }

      // All assigned windows should contain the timestamp
      for window in &windows {
        prop_assert!(window.contains(timestamp));
      }
    }
  }

  #[test]
  fn prop_sliding_assigner_windows_overlap(
    timestamp in datetime_strategy(),
    size in duration_strategy(),
    slide in duration_strategy()
  ) {
    if size.as_millis() > 0 && slide.as_millis() > 0 && size > slide {
      let assigner = SlidingWindowAssigner::new(size, slide);
      let windows = assigner.assign_windows(timestamp);

      // Adjacent windows in sliding assigner should overlap
      if windows.len() > 1 {
        for i in 0..windows.len() - 1 {
          prop_assert!(windows[i].intersects(&windows[i + 1]));
        }
      }
    }
  }

  #[test]
  fn prop_count_assigner_assigns_single_window(
    timestamp in datetime_strategy(),
    count in 1usize..=1000usize
  ) {
    let assigner = CountWindowAssigner::new(count);
    let windows = assigner.assign_windows(timestamp);

    // Count assigner should always assign exactly one window
    prop_assert_eq!(windows.len(), 1);
    prop_assert_eq!(windows[0].max_count(), count);
  }

  #[test]
  fn prop_global_assigner_assigns_single_global_window(
    timestamp in datetime_strategy()
  ) {
    let assigner = GlobalWindowAssigner::new();
    let windows = assigner.assign_windows(timestamp);

    // Global assigner should always assign exactly one global window
    prop_assert_eq!(windows.len(), 1);
    prop_assert!(matches!(windows[0], GlobalWindow));

    // Global window should contain any timestamp
    prop_assert!(windows[0].contains(timestamp));
  }

  // ==========================================================================
  // Watermark Property Tests
  // ==========================================================================

  #[test]
  fn prop_watermark_advance_monotonic(
    initial_ts in datetime_strategy(),
    later_ts in datetime_strategy()
  ) {
    let mut wm = Watermark::new(initial_ts);
    let initial = wm.timestamp;

    if later_ts > initial_ts {
      let advanced = wm.advance(later_ts);
      prop_assert!(advanced);
      prop_assert!(wm.timestamp >= initial);
      prop_assert_eq!(wm.timestamp, later_ts);
    } else {
      let advanced = wm.advance(later_ts);
      prop_assert!(!advanced);
      prop_assert_eq!(wm.timestamp, initial);
    }
  }

  #[test]
  fn prop_watermark_advance_idempotent(
    ts in datetime_strategy()
  ) {
    let mut wm = Watermark::new(ts);
    let initial = wm.timestamp;

    // Advancing to the same timestamp should not change it
    let advanced1 = wm.advance(ts);
    prop_assert!(!advanced1);
    prop_assert_eq!(wm.timestamp, initial);

    // Advancing again should still not change it
    let advanced2 = wm.advance(ts);
    prop_assert!(!advanced2);
    prop_assert_eq!(wm.timestamp, initial);
  }

  #[test]
  fn prop_watermark_ordering_consistent(
    ts1 in datetime_strategy(),
    ts2 in datetime_strategy()
  ) {
    let wm1 = Watermark::new(ts1);
    let wm2 = Watermark::new(ts2);

    match ts1.cmp(&ts2) {
      std::cmp::Ordering::Less => prop_assert!(wm1 < wm2),
      std::cmp::Ordering::Greater => prop_assert!(wm1 > wm2),
      std::cmp::Ordering::Equal => prop_assert_eq!(wm1, wm2),
    }
  }

  #[test]
  fn prop_watermark_min_max_ordering() {
    let min_wm = Watermark::min();
    let max_wm = Watermark::max();
    let any_wm = Watermark::new(Utc::now());

    prop_assert!(min_wm <= any_wm);
    prop_assert!(any_wm <= max_wm);
    prop_assert!(min_wm < max_wm);
    prop_assert!(max_wm.is_end_of_stream());
    prop_assert!(!min_wm.is_end_of_stream());
  }

  // ==========================================================================
  // Watermark Generator Property Tests
  // ==========================================================================

  #[test]
  fn prop_monotonic_generator_monotonic(
    ts1 in datetime_strategy(),
    ts2 in datetime_strategy(),
    ts3 in datetime_strategy()
  ) {
    let mut generator = MonotonicWatermarkGenerator::new();

    // Process events in order
    let timestamps = vec![ts1, ts2, ts3];
    let mut sorted = timestamps.clone();
    sorted.sort();

    let mut last_watermark = Watermark::min();
    for ts in sorted {
      if let Some(wm) = generator.on_event(ts) {
        prop_assert!(wm.timestamp >= last_watermark.timestamp);
        last_watermark = wm;
      }
    }
  }

  #[test]
  fn prop_bounded_out_of_orderness_watermark_bounded(
    max_delay in duration_strategy(),
    timestamps in proptest::collection::vec(datetime_strategy(), 1..=100)
  ) {
    if max_delay.as_millis() > 0 {
      let mut generator = BoundedOutOfOrdernessGenerator::new(max_delay);

      let mut max_ts = None;
      for ts in &timestamps {
        generator.on_event(*ts);
        max_ts = Some(max_ts.map(|m: DateTime<Utc>| m.max(*ts)).unwrap_or(*ts));
      }

      let current_wm = generator.current_watermark();
      if let Some(max) = max_ts {
        let expected_wm = max - ChronoDuration::from_std(max_delay).unwrap_or(ChronoDuration::zero());
        prop_assert!(current_wm.timestamp <= expected_wm);
      }
    }
  }

  // ==========================================================================
  // Late Data Handler Property Tests
  // ==========================================================================

  #[test]
  fn prop_late_data_handler_on_time_elements_processed(
    element_ts in datetime_strategy(),
    watermark_ts in datetime_strategy()
  ) {
    let mut handler = LateDataHandler::drop_late();
    let watermark = Watermark::new(watermark_ts);

    if element_ts >= watermark_ts {
      let result = handler.evaluate(element_ts, &watermark, 42);
      prop_assert!(matches!(result, LateDataResult::OnTime(42)));
      prop_assert!(result.should_process());
      prop_assert_eq!(result.into_processable(), Some(42));
    }
  }

  #[test]
  fn prop_late_data_handler_within_lateness_processed(
    watermark_ts in datetime_strategy(),
    lateness in duration_strategy()
  ) {
    if lateness.as_millis() > 0 {
      let mut handler = LateDataHandler::with_allowed_lateness(lateness);
      let watermark = Watermark::new(watermark_ts);

      // Create element that is late but within allowed lateness
      let lateness_chrono = ChronoDuration::from_std(lateness).unwrap_or(ChronoDuration::zero());
      if let Some(element_ts) = watermark_ts.checked_sub_signed(lateness_chrono / 2) {
        let result = handler.evaluate(element_ts, &watermark, 42);
        prop_assert!(result.should_process());
      }
    }
  }

  #[test]
  fn prop_late_data_handler_stats_accumulate(
    watermark_ts in datetime_strategy(),
    num_on_time in 0usize..=100,
    num_late in 0usize..=100
  ) {
    let mut handler = LateDataHandler::drop_late();
    let watermark = Watermark::new(watermark_ts);

    // Add on-time elements
    for i in 0..num_on_time {
      let ts = watermark_ts + ChronoDuration::seconds(i as i64);
      handler.evaluate(ts, &watermark, i);
    }

    // Add late elements
    for i in 0..num_late {
      let ts = watermark_ts - ChronoDuration::seconds((i + 1) as i64);
      handler.evaluate(ts, &watermark, i);
    }

    let stats = handler.stats();
    prop_assert_eq!(stats.on_time, num_on_time as u64);
    prop_assert_eq!(stats.dropped, num_late as u64);
  }

  // ==========================================================================
  // Window State Property Tests
  // ==========================================================================

  #[test]
  fn prop_window_state_add_increases_count(
    start in datetime_strategy(),
    duration in duration_strategy(),
    num_elements in 0usize..=100
  ) {
    let end = start + ChronoDuration::from_std(duration).unwrap_or(ChronoDuration::zero());
    if end > start {
      let window = TimeWindow::new(start, end);
      let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

      for i in 0..num_elements {
        let ts = start + ChronoDuration::seconds(i as i64);
        state.add(ts, i as i32);
      }

      prop_assert_eq!(state.count, num_elements);
      prop_assert_eq!(state.elements().len(), num_elements);
      prop_assert_eq!(state.values().len(), num_elements);
    }
  }

  #[test]
  fn prop_window_state_clear_resets(
    start in datetime_strategy(),
    duration in duration_strategy(),
    num_elements in 1usize..=100
  ) {
    let end = start + ChronoDuration::from_std(duration).unwrap_or(ChronoDuration::zero());
    if end > start {
      let window = TimeWindow::new(start, end);
      let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

      // Add elements
      for i in 0..num_elements {
        let ts = start + ChronoDuration::seconds(i as i64);
        state.add(ts, i as i32);
      }

      prop_assert_eq!(state.count, num_elements);

      // Clear
      state.clear();

      prop_assert_eq!(state.count, 0);
      prop_assert!(state.elements().is_empty());
      prop_assert!(state.values().is_empty());
    }
  }

  #[test]
  fn prop_window_state_gc_requires_triggered(
    start in datetime_strategy(),
    duration in duration_strategy(),
    watermark_ts in datetime_strategy()
  ) {
    let end = start + ChronoDuration::from_std(duration).unwrap_or(ChronoDuration::zero());
    if end > start {
      let window = TimeWindow::new(start, end);
      let mut state: WindowState<TimeWindow, i32> = WindowState::new(window);

      let watermark = Watermark::new(watermark_ts);

      // Non-triggered window should not be GC'd
      prop_assert!(!state.can_be_gc(&watermark, ChronoDuration::zero()));

      // After triggering, it might be GC'd
      state.mark_triggered();
      // Can't assert it will be GC'd without knowing watermark position
      // But we can assert the method doesn't panic
      let _ = state.can_be_gc(&watermark, ChronoDuration::zero());
    }
  }
}
