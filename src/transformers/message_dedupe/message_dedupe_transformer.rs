//! Message deduplication transformer implementation.
//!
//! Provides exactly-once processing semantics by filtering duplicate messages
//! based on their unique identifiers.

use crate::error::ErrorStrategy;
use crate::message::{Message, MessageId};
use crate::transformer::TransformerConfig;
use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// Strategy for limiting the deduplication window.
#[derive(Clone, Debug)]
pub enum DeduplicationWindow {
  /// Keep all seen IDs indefinitely (unbounded memory).
  Unbounded,

  /// Keep only the last N message IDs.
  Count(usize),

  /// Keep IDs seen within the last duration.
  Time(Duration),

  /// Keep IDs with both count and time limits (uses whichever is stricter).
  CountAndTime {
    /// Maximum number of IDs to keep.
    max_count: usize,
    /// Maximum age of IDs to keep.
    max_age: Duration,
  },
}

impl Default for DeduplicationWindow {
  fn default() -> Self {
    DeduplicationWindow::Count(10_000)
  }
}

/// Entry in the deduplication cache, tracking the ID and when it was seen.
#[derive(Clone, Debug)]
pub(super) struct CacheEntry {
  pub(super) id: MessageId,
  pub(super) seen_at: Instant,
}

impl PartialEq for CacheEntry {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl Eq for CacheEntry {}

impl Hash for CacheEntry {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.id.hash(state);
  }
}

/// A transformer that filters duplicate messages based on their unique IDs.
///
/// This transformer maintains a cache of recently seen message IDs and filters
/// out any messages whose ID has already been seen within the configured window.
///
/// # Window Strategies
///
/// - `Unbounded`: Keep all IDs (careful: memory grows without bound)
/// - `Count(n)`: Keep the last n IDs (FIFO eviction)
/// - `Time(d)`: Keep IDs seen within duration d
/// - `CountAndTime`: Combine both limits
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::message_dedupe::{MessageDedupeTransformer, DeduplicationWindow};
/// use streamweave::message::{Message, MessageId};
///
/// // Create a deduplicator keeping the last 1000 message IDs
/// let deduper = MessageDedupeTransformer::<i32>::new()
///     .with_window(DeduplicationWindow::Count(1000));
/// ```
#[derive(Debug)]
pub struct MessageDedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The deduplication window configuration.
  pub(super) window: DeduplicationWindow,

  /// Set of seen message IDs for O(1) lookup.
  pub(super) seen_ids: HashSet<MessageId>,

  /// Ordered queue of cache entries for eviction (FIFO for count-based, or time-ordered).
  pub(super) eviction_queue: VecDeque<CacheEntry>,

  /// Transformer configuration.
  pub config: TransformerConfig<Message<T>>,

  /// Statistics: total messages processed.
  messages_processed: u64,

  /// Statistics: duplicates filtered.
  duplicates_filtered: u64,
}

impl<T> Default for MessageDedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> MessageDedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Create a new message deduplication transformer with default settings.
  #[must_use]
  pub fn new() -> Self {
    Self {
      window: DeduplicationWindow::default(),
      seen_ids: HashSet::new(),
      eviction_queue: VecDeque::new(),
      config: TransformerConfig::default(),
      messages_processed: 0,
      duplicates_filtered: 0,
    }
  }

  /// Set the deduplication window.
  #[must_use]
  pub fn with_window(mut self, window: DeduplicationWindow) -> Self {
    self.window = window;
    self
  }

  /// Set the error strategy.
  #[must_use]
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Message<T>>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Set the transformer name.
  #[must_use]
  pub fn with_name(mut self, name: impl Into<String>) -> Self {
    self.config.name = Some(name.into());
    self
  }

  /// Get the current window configuration.
  #[must_use]
  pub fn window(&self) -> &DeduplicationWindow {
    &self.window
  }

  /// Get the number of currently cached IDs.
  #[must_use]
  pub fn cache_size(&self) -> usize {
    self.seen_ids.len()
  }

  /// Get the total number of messages processed.
  #[must_use]
  pub fn messages_processed(&self) -> u64 {
    self.messages_processed
  }

  /// Get the number of duplicates that were filtered.
  #[must_use]
  pub fn duplicates_filtered(&self) -> u64 {
    self.duplicates_filtered
  }

  /// Get the deduplication ratio (duplicates / total).
  #[must_use]
  pub fn deduplication_ratio(&self) -> f64 {
    if self.messages_processed == 0 {
      0.0
    } else {
      self.duplicates_filtered as f64 / self.messages_processed as f64
    }
  }

  /// Clear all cached IDs and reset statistics.
  pub fn clear(&mut self) {
    self.seen_ids.clear();
    self.eviction_queue.clear();
    self.messages_processed = 0;
    self.duplicates_filtered = 0;
  }

  /// Check if a message ID has been seen (is a duplicate).
  #[must_use]
  pub fn is_duplicate(&self, id: &MessageId) -> bool {
    self.seen_ids.contains(id)
  }

  /// Record a message ID as seen and return whether it was a duplicate.
  ///
  /// This also performs cache eviction based on the window configuration.
  pub fn record_id(&mut self, id: MessageId) -> bool {
    self.messages_processed += 1;

    // Evict old entries based on window configuration
    self.evict_expired();

    // Check if duplicate
    if self.seen_ids.contains(&id) {
      self.duplicates_filtered += 1;
      return true;
    }

    // Add to cache
    let entry = CacheEntry {
      id: id.clone(),
      seen_at: Instant::now(),
    };
    self.seen_ids.insert(id);
    self.eviction_queue.push_back(entry);

    // Evict if over count limit
    self.evict_by_count();

    false
  }

  /// Evict entries that have expired based on time.
  fn evict_expired(&mut self) {
    let max_age = match &self.window {
      DeduplicationWindow::Time(duration) => Some(*duration),
      DeduplicationWindow::CountAndTime { max_age, .. } => Some(*max_age),
      _ => None,
    };

    if let Some(max_age) = max_age {
      let now = Instant::now();
      while let Some(entry) = self.eviction_queue.front() {
        if now.duration_since(entry.seen_at) > max_age {
          let removed = self.eviction_queue.pop_front().unwrap();
          self.seen_ids.remove(&removed.id);
        } else {
          // Queue is ordered by time, so we can stop here
          break;
        }
      }
    }
  }

  /// Evict entries if over the count limit.
  fn evict_by_count(&mut self) {
    let max_count = match &self.window {
      DeduplicationWindow::Count(count) => Some(*count),
      DeduplicationWindow::CountAndTime { max_count, .. } => Some(*max_count),
      _ => None,
    };

    if let Some(max_count) = max_count {
      while self.eviction_queue.len() > max_count {
        if let Some(removed) = self.eviction_queue.pop_front() {
          self.seen_ids.remove(&removed.id);
        }
      }
    }
  }
}

impl<T> Clone for MessageDedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    // Clone creates a fresh transformer with the same configuration
    // but empty cache (intentional for parallel processing)
    Self {
      window: self.window.clone(),
      seen_ids: HashSet::new(),
      eviction_queue: VecDeque::new(),
      config: self.config.clone(),
      messages_processed: 0,
      duplicates_filtered: 0,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::thread;

  #[test]
  fn test_new_default() {
    let deduper = MessageDedupeTransformer::<i32>::new();
    assert_eq!(deduper.cache_size(), 0);
    assert_eq!(deduper.messages_processed(), 0);
    assert_eq!(deduper.duplicates_filtered(), 0);
  }

  #[test]
  fn test_record_unique_ids() {
    let mut deduper = MessageDedupeTransformer::<i32>::new();

    assert!(!deduper.record_id(MessageId::new_sequence(1)));
    assert!(!deduper.record_id(MessageId::new_sequence(2)));
    assert!(!deduper.record_id(MessageId::new_sequence(3)));

    assert_eq!(deduper.cache_size(), 3);
    assert_eq!(deduper.messages_processed(), 3);
    assert_eq!(deduper.duplicates_filtered(), 0);
  }

  #[test]
  fn test_detect_duplicates() {
    let mut deduper = MessageDedupeTransformer::<i32>::new();

    assert!(!deduper.record_id(MessageId::new_sequence(1)));
    assert!(deduper.record_id(MessageId::new_sequence(1))); // duplicate
    assert!(!deduper.record_id(MessageId::new_sequence(2)));
    assert!(deduper.record_id(MessageId::new_sequence(1))); // duplicate again

    assert_eq!(deduper.cache_size(), 2);
    assert_eq!(deduper.messages_processed(), 4);
    assert_eq!(deduper.duplicates_filtered(), 2);
  }

  #[test]
  fn test_count_window_eviction() {
    let mut deduper =
      MessageDedupeTransformer::<i32>::new().with_window(DeduplicationWindow::Count(3));

    deduper.record_id(MessageId::new_sequence(1));
    deduper.record_id(MessageId::new_sequence(2));
    deduper.record_id(MessageId::new_sequence(3));
    assert_eq!(deduper.cache_size(), 3);

    // Adding a 4th should evict the 1st
    deduper.record_id(MessageId::new_sequence(4));
    assert_eq!(deduper.cache_size(), 3);

    // ID 1 should no longer be detected as duplicate
    assert!(!deduper.record_id(MessageId::new_sequence(1)));
  }

  #[test]
  fn test_time_window_eviction() {
    let mut deduper = MessageDedupeTransformer::<i32>::new()
      .with_window(DeduplicationWindow::Time(Duration::from_millis(100)));

    deduper.record_id(MessageId::new_sequence(1));
    assert_eq!(deduper.cache_size(), 1);

    // Should still be there immediately
    assert!(deduper.is_duplicate(&MessageId::new_sequence(1)));

    // Wait for expiration
    thread::sleep(Duration::from_millis(150));

    // Recording a new ID should trigger eviction
    deduper.record_id(MessageId::new_sequence(2));

    // ID 1 should be expired
    assert!(!deduper.is_duplicate(&MessageId::new_sequence(1)));
  }

  #[test]
  fn test_deduplication_ratio() {
    let mut deduper = MessageDedupeTransformer::<i32>::new();

    // 0 messages = 0.0 ratio
    assert_eq!(deduper.deduplication_ratio(), 0.0);

    // 4 messages, 2 duplicates = 0.5 ratio
    deduper.record_id(MessageId::new_sequence(1));
    deduper.record_id(MessageId::new_sequence(1)); // dup
    deduper.record_id(MessageId::new_sequence(2));
    deduper.record_id(MessageId::new_sequence(2)); // dup

    assert_eq!(deduper.deduplication_ratio(), 0.5);
  }

  #[test]
  fn test_clear() {
    let mut deduper = MessageDedupeTransformer::<i32>::new();

    deduper.record_id(MessageId::new_sequence(1));
    deduper.record_id(MessageId::new_sequence(2));
    assert_eq!(deduper.cache_size(), 2);

    deduper.clear();
    assert_eq!(deduper.cache_size(), 0);
    assert_eq!(deduper.messages_processed(), 0);
    assert_eq!(deduper.duplicates_filtered(), 0);
  }

  #[test]
  fn test_with_name() {
    let deduper = MessageDedupeTransformer::<i32>::new().with_name("my-deduper");
    assert_eq!(deduper.config.name, Some("my-deduper".to_string()));
  }

  #[test]
  fn test_clone_has_empty_cache() {
    let mut deduper = MessageDedupeTransformer::<i32>::new();
    deduper.record_id(MessageId::new_sequence(1));
    deduper.record_id(MessageId::new_sequence(2));

    let cloned = deduper.clone();
    // Original has cached IDs
    assert_eq!(deduper.cache_size(), 2);
    // Clone starts fresh (for parallel processing)
    assert_eq!(cloned.cache_size(), 0);
  }

  #[test]
  fn test_different_id_types() {
    let mut deduper = MessageDedupeTransformer::<String>::new();

    // UUID
    deduper.record_id(MessageId::new_uuid());
    deduper.record_id(MessageId::new_uuid());

    // Sequence
    deduper.record_id(MessageId::new_sequence(1));

    // Custom
    deduper.record_id(MessageId::new_custom("custom-id-1"));
    assert!(deduper.record_id(MessageId::new_custom("custom-id-1"))); // dup

    // Content hash
    deduper.record_id(MessageId::from_content(b"hello"));
    assert!(deduper.record_id(MessageId::from_content(b"hello"))); // dup

    assert_eq!(deduper.duplicates_filtered(), 2);
  }
}
