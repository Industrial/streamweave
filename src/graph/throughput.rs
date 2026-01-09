//! # Throughput Monitoring Module
//!
//! This module provides throughput monitoring functionality for hybrid execution mode.
//! It tracks items/second processed across the graph and calculates rolling averages
//! to smooth out spikes.
//!
//! ## Usage
//!
//! ```rust
//! use crate::graph::throughput::ThroughputMonitor;
//!
//! let monitor = ThroughputMonitor::new(Duration::from_secs(1));
//!
//! // Increment item count as items are processed
//! monitor.increment_item_count();
//!
//! // Calculate current throughput
//! let throughput = monitor.calculate_throughput();
//! println!("Throughput: {} items/second", throughput);
//! ```

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// A monitor that tracks throughput (items/second) with rolling average calculation.
///
/// This monitor uses atomic counters for thread-safe item counting and maintains
/// a rolling window of samples to calculate smoothed throughput values.
pub struct ThroughputMonitor {
  /// Total item count (atomic for thread-safe access)
  item_count: Arc<AtomicU64>,
  /// Start time of monitoring
  start_time: Arc<RwLock<Instant>>,
  /// Window size for rolling average calculation
  window_size: Duration,
  /// Samples for rolling average: (timestamp, item_count)
  samples: Arc<RwLock<VecDeque<(Instant, u64)>>>,
  /// Last reset time (for window-based calculation)
  last_reset: Arc<RwLock<Instant>>,
}

impl ThroughputMonitor {
  /// Create a new `ThroughputMonitor` with the specified window size.
  ///
  /// # Arguments
  ///
  /// * `window_size` - Duration of the rolling window for average calculation
  ///
  /// # Returns
  ///
  /// A new `ThroughputMonitor` instance.
  ///
  /// # Example
  ///
  /// ```rust
  /// use crate::graph::throughput::ThroughputMonitor;
  /// use std::time::Duration;
  ///
  /// let monitor = ThroughputMonitor::new(Duration::from_secs(1));
  /// ```
  pub fn new(window_size: Duration) -> Self {
    let now = Instant::now();
    Self {
      item_count: Arc::new(AtomicU64::new(0)),
      start_time: Arc::new(RwLock::new(now)),
      window_size,
      samples: Arc::new(RwLock::new(VecDeque::new())),
      last_reset: Arc::new(RwLock::new(now)),
    }
  }

  /// Create a new `ThroughputMonitor` with default window size (1 second).
  #[allow(clippy::should_implement_trait)]
  pub fn default() -> Self {
    Self::new(Duration::from_secs(1))
  }

  /// Increment the item count by 1.
  ///
  /// This method is thread-safe and can be called from any node or thread.
  pub fn increment_item_count(&self) {
    self.item_count.fetch_add(1, Ordering::Relaxed);
  }

  /// Increment the item count by the specified amount.
  ///
  /// # Arguments
  ///
  /// * `count` - Number of items to add
  pub fn increment_by(&self, count: u64) {
    self.item_count.fetch_add(count, Ordering::Relaxed);
  }

  /// Get the current total item count.
  ///
  /// # Returns
  ///
  /// The total number of items processed since monitoring started.
  pub fn item_count(&self) -> u64 {
    self.item_count.load(Ordering::Relaxed)
  }

  /// Calculate the current throughput (items/second) using a rolling average.
  ///
  /// This method maintains a rolling window of samples and calculates the
  /// average throughput over the window period.
  ///
  /// # Returns
  ///
  /// The current throughput in items/second (as a floating-point value).
  pub async fn calculate_throughput(&self) -> f64 {
    let now = Instant::now();
    let current_count = self.item_count.load(Ordering::Relaxed);

    // Add current sample
    let mut samples = self.samples.write().await;
    samples.push_back((now, current_count));

    // Remove samples outside the window
    let window_start = now.checked_sub(self.window_size).unwrap_or(now);
    while samples
      .front()
      .map(|(ts, _)| *ts < window_start)
      .unwrap_or(false)
    {
      samples.pop_front();
    }

    // Calculate throughput from window
    if samples.len() < 2 {
      // Not enough samples, return 0 or instantaneous rate
      return 0.0;
    }

    let (first_time, first_count) = samples.front().unwrap();
    let (last_time, last_count) = samples.back().unwrap();

    let time_diff = last_time.duration_since(*first_time);
    if time_diff.is_zero() {
      return 0.0;
    }

    let count_diff = last_count.saturating_sub(*first_count);
    count_diff as f64 / time_diff.as_secs_f64()
  }

  /// Calculate the instantaneous throughput (items/second) since the last reset.
  ///
  /// This method calculates throughput based on the time since the last reset
  /// or since monitoring started.
  ///
  /// # Returns
  ///
  /// The instantaneous throughput in items/second.
  pub async fn calculate_instantaneous_throughput(&self) -> f64 {
    let now = Instant::now();
    let current_count = self.item_count.load(Ordering::Relaxed);
    let start_time = *self.start_time.read().await;
    let last_reset = *self.last_reset.read().await;

    // Use the more recent of start_time or last_reset
    let base_time = if last_reset > start_time {
      last_reset
    } else {
      start_time
    };
    let elapsed = now.duration_since(base_time);

    if elapsed.is_zero() {
      return 0.0;
    }

    // Get count at base_time (approximate from samples or use 0)
    let base_count = if last_reset > start_time {
      // Reset happened, count should be relative to reset
      // For simplicity, assume count was reset to 0
      0
    } else {
      0 // Start count
    };

    let count_diff = current_count.saturating_sub(base_count);
    count_diff as f64 / elapsed.as_secs_f64()
  }

  /// Reset the throughput monitor.
  ///
  /// This resets the item count and clears the sample history.
  /// Useful for starting a new measurement period.
  pub async fn reset(&self) {
    self.item_count.store(0, Ordering::Relaxed);
    *self.start_time.write().await = Instant::now();
    *self.last_reset.write().await = Instant::now();
    self.samples.write().await.clear();
  }

  /// Get the window size for rolling average calculation.
  ///
  /// # Returns
  ///
  /// The window size duration.
  pub fn window_size(&self) -> Duration {
    self.window_size
  }

  /// Get statistics about the throughput monitor.
  ///
  /// # Returns
  ///
  /// A tuple of (total_items, window_samples_count, elapsed_time).
  pub async fn statistics(&self) -> (u64, usize, Duration) {
    let total_items = self.item_count.load(Ordering::Relaxed);
    let samples_count = self.samples.read().await.len();
    let elapsed = self.start_time.read().await.elapsed();
    (total_items, samples_count, elapsed)
  }
}

impl Clone for ThroughputMonitor {
  fn clone(&self) -> Self {
    Self {
      item_count: Arc::clone(&self.item_count),
      start_time: Arc::clone(&self.start_time),
      window_size: self.window_size,
      samples: Arc::clone(&self.samples),
      last_reset: Arc::clone(&self.last_reset),
    }
  }
}
