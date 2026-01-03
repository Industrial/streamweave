//! Message deduplication transformer implementation.
//!
//! Provides exactly-once processing semantics by filtering duplicate messages
//! based on their unique identifiers.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::message::{Message, MessageId};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::pin::Pin;
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
/// use streamweave::transformers::{MessageDedupeTransformer, DeduplicationWindow};
/// use crate::message::{Message, MessageId};
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

impl<T> Input for MessageDedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = Message<T>;
  type InputStream = Pin<Box<dyn futures::Stream<Item = Self::Input> + Send>>;
}

impl<T> Output for MessageDedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = Message<T>;
  type OutputStream = Pin<Box<dyn futures::Stream<Item = Self::Output> + Send>>;
}

#[async_trait]
impl<T> Transformer for MessageDedupeTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (crate::message::Message<T>,);
  type OutputPorts = (crate::message::Message<T>,);
  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    // Move the state into the stream for filtering
    let seen_ids = std::mem::take(&mut self.seen_ids);
    let eviction_queue = std::mem::take(&mut self.eviction_queue);
    let window = self.window.clone();

    Box::pin(stream::unfold(
      (input, seen_ids, eviction_queue, window),
      |(mut input, mut seen_ids, mut eviction_queue, window)| async move {
        loop {
          match input.next().await {
            Some(msg) => {
              let id = msg.id().clone();

              // Check if duplicate
              if seen_ids.contains(&id) {
                // Skip duplicate, continue to next item
                continue;
              }

              // Record this ID
              let entry = CacheEntry {
                id: id.clone(),
                seen_at: std::time::Instant::now(),
              };
              seen_ids.insert(id);
              eviction_queue.push_back(entry);

              // Evict if over count limit
              let max_count = match &window {
                DeduplicationWindow::Count(count) => Some(*count),
                DeduplicationWindow::CountAndTime { max_count, .. } => Some(*max_count),
                _ => None,
              };

              if let Some(max_count) = max_count {
                while eviction_queue.len() > max_count {
                  if let Some(removed) = eviction_queue.pop_front() {
                    seen_ids.remove(&removed.id);
                  }
                }
              }

              return Some((msg, (input, seen_ids, eviction_queue, window)));
            }
            None => return None,
          }
        }
      },
    ))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Message<T>>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Message<T>> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Message<T>> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Message<T>>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Message<T>>) -> ErrorContext<Message<T>> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "message_dedupe_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
