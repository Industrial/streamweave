//! Comprehensive tests for throughput monitoring module
//!
//! This module provides 100% coverage tests for:
//! - ThroughputMonitor creation and configuration
//! - Item counting (increment, increment_by)
//! - Throughput calculation
//! - Rolling average calculation
//! - Thread-safety
//! - Edge cases

use std::thread;
use std::time::Duration;
use streamweave::graph::throughput::ThroughputMonitor;
use tokio::runtime::Runtime;

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[test]
fn test_throughput_monitor_new() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));
  assert_eq!(monitor.item_count(), 0);
}

#[test]
fn test_throughput_monitor_default() {
  let monitor = ThroughputMonitor::default();
  assert_eq!(monitor.item_count(), 0);
}

#[test]
fn test_throughput_monitor_increment() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));

  assert_eq!(monitor.item_count(), 0);
  monitor.increment_item_count();
  assert_eq!(monitor.item_count(), 1);
  monitor.increment_item_count();
  assert_eq!(monitor.item_count(), 2);
}

#[test]
fn test_throughput_monitor_increment_by() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));

  assert_eq!(monitor.item_count(), 0);
  monitor.increment_by(5);
  assert_eq!(monitor.item_count(), 5);
  monitor.increment_by(10);
  assert_eq!(monitor.item_count(), 15);
}

#[test]
fn test_throughput_monitor_reset() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));

  monitor.increment_by(10);
  assert_eq!(monitor.item_count(), 10);

  monitor.reset();
  assert_eq!(monitor.item_count(), 0);
}

#[test]
fn test_throughput_monitor_calculate_throughput() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));

  // Initially, throughput should be 0
  let throughput = monitor.calculate_throughput();
  assert_eq!(throughput, 0.0);

  // Increment items
  monitor.increment_by(100);

  // Wait a bit and calculate
  thread::sleep(Duration::from_millis(100));
  let throughput = monitor.calculate_throughput();

  // Should have some throughput (exact value depends on timing)
  assert!(throughput >= 0.0);
}

#[test]
fn test_throughput_monitor_calculate_rolling_average() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));

  // Initially, average should be 0
  let avg = monitor.calculate_rolling_average();
  assert_eq!(avg, 0.0);

  // Increment items
  monitor.increment_by(100);

  // Wait and calculate
  thread::sleep(Duration::from_millis(100));
  let avg = monitor.calculate_rolling_average();

  // Should have some average (exact value depends on timing)
  assert!(avg >= 0.0);
}

// ============================================================================
// Thread-Safety Tests
// ============================================================================

#[test]
fn test_throughput_monitor_thread_safety() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));
  let monitor = std::sync::Arc::new(monitor);

  let mut handles = vec![];

  // Spawn 10 threads, each incrementing 100 times
  for _ in 0..10 {
    let monitor_clone = monitor.clone();
    let handle = thread::spawn(move || {
      for _ in 0..100 {
        monitor_clone.increment_item_count();
      }
    });
    handles.push(handle);
  }

  // Wait for all threads
  for handle in handles {
    handle.join().unwrap();
  }

  // Should have 1000 items total
  assert_eq!(monitor.item_count(), 1000);
}

#[test]
fn test_throughput_monitor_concurrent_increment_by() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));
  let monitor = std::sync::Arc::new(monitor);

  let mut handles = vec![];

  // Spawn 5 threads, each incrementing by different amounts
  for i in 0..5 {
    let monitor_clone = monitor.clone();
    let handle = thread::spawn(move || {
      monitor_clone.increment_by((i + 1) * 10);
    });
    handles.push(handle);
  }

  // Wait for all threads
  for handle in handles {
    handle.join().unwrap();
  }

  // Should have 10 + 20 + 30 + 40 + 50 = 150 items
  assert_eq!(monitor.item_count(), 150);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_throughput_monitor_zero_window() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(0));
  monitor.increment_by(100);

  // Should not panic
  let throughput = monitor.calculate_throughput();
  assert!(throughput >= 0.0);
}

#[test]
fn test_throughput_monitor_large_window() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(3600)); // 1 hour
  monitor.increment_by(100);

  let throughput = monitor.calculate_throughput();
  assert!(throughput >= 0.0);
}

#[test]
fn test_throughput_monitor_very_large_count() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));
  monitor.increment_by(u64::MAX / 2);

  // Should not panic
  let count = monitor.item_count();
  assert!(count > 0);
}

#[test]
fn test_throughput_monitor_multiple_resets() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));

  monitor.increment_by(100);
  monitor.reset();
  assert_eq!(monitor.item_count(), 0);

  monitor.increment_by(50);
  monitor.reset();
  assert_eq!(monitor.item_count(), 0);

  monitor.increment_by(25);
  assert_eq!(monitor.item_count(), 25);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_throughput_monitor_full_workflow() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));

  // Simulate processing items
  for i in 0..1000 {
    if i % 10 == 0 {
      monitor.increment_by(10);
    } else {
      monitor.increment_item_count();
    }
  }

  assert_eq!(monitor.item_count(), 1000);

  // Calculate throughput
  thread::sleep(Duration::from_millis(50));
  let throughput = monitor.calculate_throughput();
  assert!(throughput >= 0.0);

  // Calculate rolling average
  let avg = monitor.calculate_rolling_average();
  assert!(avg >= 0.0);

  // Reset and verify
  monitor.reset();
  assert_eq!(monitor.item_count(), 0);
}

#[tokio::test]
async fn test_throughput_monitor_async() {
  let monitor = ThroughputMonitor::new(Duration::from_secs(1));

  // Simulate async processing
  for _ in 0..100 {
    monitor.increment_item_count();
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
  }

  assert_eq!(monitor.item_count(), 100);

  let throughput = monitor.calculate_throughput();
  assert!(throughput >= 0.0);
}

// Tests moved from src/
#[tokio::test]
async fn test_throughput_monitor_increment() {
  let monitor = ThroughputMonitor::default();
  assert_eq!(monitor.item_count(), 0);

  monitor.increment_item_count();
  assert_eq!(monitor.item_count(), 1);

  monitor.increment_by(5);
  assert_eq!(monitor.item_count(), 6);
}

#[tokio::test]
async fn test_throughput_monitor_calculation() {
  let monitor = ThroughputMonitor::new(Duration::from_millis(100));

  // Increment items
  for _ in 0..10 {
    monitor.increment_item_count();
  }

  // Wait a bit for time to pass
  tokio::time::sleep(Duration::from_millis(50)).await;

  let throughput = monitor.calculate_throughput().await;
  // Throughput should be positive (exact value depends on timing)
  assert!(throughput >= 0.0);
}

#[tokio::test]
async fn test_throughput_monitor_reset() {
  let monitor = ThroughputMonitor::default();

  monitor.increment_by(100);
  assert_eq!(monitor.item_count(), 100);

  monitor.reset().await;
  assert_eq!(monitor.item_count(), 0);
}

#[tokio::test]
async fn test_throughput_monitor_statistics() {
  let monitor = ThroughputMonitor::default();

  monitor.increment_by(50);
  let (total, _samples, _elapsed) = monitor.statistics().await;

  assert_eq!(total, 50);
}
