//! # Batching Module
//!
//! This module provides batching functionality for distributed execution mode.
//! It buffers items and sends them as batches to reduce network overhead.
//!
//! ## Usage
//!
//! ```rust
//! use streamweave_graph::batching::{BatchBuffer, BatchConfig};
//! use bytes::Bytes;
//!
//! let config = BatchConfig::new(100, 1000); // 100 items or 1 second
//! let mut buffer = BatchBuffer::new(config);
//!
//! // Add items
//! buffer.add(Bytes::from("item1"));
//! buffer.add(Bytes::from("item2"));
//!
//! // Check if should flush
//! if buffer.should_flush() {
//!     let batch = buffer.flush();
//!     // Send batch...
//! }
//! ```

use crate::channels::ChannelItem;
use crate::execution::BatchConfig;
use bytes::Bytes;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Error types for batching operations.
#[derive(Error, Debug)]
pub enum BatchingError {
  #[error("Batch buffer is full")]
  BufferFull,
  #[error("Serialization error: {0}")]
  SerializationError(String),
  #[error("Invalid batch configuration: {0}")]
  InvalidConfig(String),
}

/// A buffer that collects items and flushes when batch conditions are met.
///
/// This buffer maintains a collection of items and tracks when to flush
/// based on batch size or timeout.
pub struct BatchBuffer {
  /// Buffered items
  items: Vec<Bytes>,
  /// Batch configuration
  config: BatchConfig,
  /// Timestamp of last flush (or creation)
  last_flush: Instant,
  /// Maximum buffer size to prevent unbounded growth
  max_buffer_size: usize,
}

impl BatchBuffer {
  /// Create a new `BatchBuffer` with the given configuration.
  ///
  /// # Arguments
  ///
  /// * `config` - Batch configuration (size and timeout)
  ///
  /// # Returns
  ///
  /// A new `BatchBuffer` instance.
  pub fn new(config: BatchConfig) -> Self {
    let max_buffer_size = config.batch_size * 10; // Allow up to 10x batch size
    Self {
      items: Vec::new(),
      config,
      last_flush: Instant::now(),
      max_buffer_size,
    }
  }

  /// Add an item to the buffer.
  ///
  /// # Arguments
  ///
  /// * `item` - The item to add
  ///
  /// # Returns
  ///
  /// `Ok(())` if the item was added, `Err(BatchingError)` if the buffer is full.
  pub fn add(&mut self, item: Bytes) -> Result<(), BatchingError> {
    if self.items.len() >= self.max_buffer_size {
      return Err(BatchingError::BufferFull);
    }
    self.items.push(item);
    Ok(())
  }

  /// Check if the buffer should be flushed.
  ///
  /// Returns `true` if:
  /// - The buffer has reached `batch_size` items, OR
  /// - The timeout (`batch_timeout_ms`) has expired
  ///
  /// # Returns
  ///
  /// `true` if the buffer should be flushed, `false` otherwise.
  pub fn should_flush(&self) -> bool {
    // Check size threshold
    if self.items.len() >= self.config.batch_size {
      return true;
    }

    // Check timeout (only if we have at least one item)
    if !self.items.is_empty() {
      let elapsed = self.last_flush.elapsed();
      let timeout = Duration::from_millis(self.config.batch_timeout_ms);
      if elapsed >= timeout {
        return true;
      }
    }

    false
  }

  /// Flush the buffer, returning all items as a batch.
  ///
  /// This clears the buffer and resets the flush timestamp.
  ///
  /// # Returns
  ///
  /// A vector of items that were in the buffer.
  pub fn flush(&mut self) -> Vec<Bytes> {
    let items = std::mem::take(&mut self.items);
    self.last_flush = Instant::now();
    items
  }

  /// Get the number of items currently in the buffer.
  ///
  /// # Returns
  ///
  /// The number of items in the buffer.
  pub fn len(&self) -> usize {
    self.items.len()
  }

  /// Check if the buffer is empty.
  ///
  /// # Returns
  ///
  /// `true` if the buffer is empty, `false` otherwise.
  pub fn is_empty(&self) -> bool {
    self.items.is_empty()
  }

  /// Get the time elapsed since the last flush.
  ///
  /// # Returns
  ///
  /// The duration since the last flush.
  pub fn time_since_flush(&self) -> Duration {
    self.last_flush.elapsed()
  }

  /// Force flush the buffer even if conditions aren't met.
  ///
  /// This is useful during shutdown to ensure all items are sent.
  ///
  /// # Returns
  ///
  /// A vector of items that were in the buffer (may be empty).
  pub fn force_flush(&mut self) -> Vec<Bytes> {
    self.flush()
  }

  /// Get a reference to the batch configuration.
  pub fn config(&self) -> &BatchConfig {
    &self.config
  }
}

/// Serialize a batch of items into a single `Bytes` buffer.
///
/// Format: `[u64: item_count][u64: item1_len][item1_bytes][u64: item2_len][item2_bytes]...`
///
/// # Arguments
///
/// * `items` - Vector of items to serialize
///
/// # Returns
///
/// Serialized batch as `Bytes`, or an error if serialization fails.
pub fn serialize_batch(items: Vec<Bytes>) -> Result<Bytes, BatchingError> {
  if items.is_empty() {
    return Ok(Bytes::new());
  }

  // Calculate total size needed
  let mut total_size = 8; // For item_count (u64)
  for item in &items {
    total_size += 8; // For item length (u64)
    total_size += item.len(); // For item data
  }

  // Allocate buffer
  let mut buffer = Vec::with_capacity(total_size);

  // Write item count
  let item_count = items.len() as u64;
  buffer.extend_from_slice(&item_count.to_le_bytes());

  // Write each item (length-prefixed)
  for item in items {
    let item_len = item.len() as u64;
    buffer.extend_from_slice(&item_len.to_le_bytes());
    buffer.extend_from_slice(&item);
  }

  Ok(buffer.into())
}

/// Deserialize a batch of items from a `Bytes` buffer.
///
/// Format: `[u64: item_count][u64: item1_len][item1_bytes][u64: item2_len][item2_bytes]...`
///
/// # Arguments
///
/// * `bytes` - Serialized batch data
///
/// # Returns
///
/// Vector of deserialized items, or an error if deserialization fails.
pub fn deserialize_batch(bytes: Bytes) -> Result<Vec<Bytes>, BatchingError> {
  if bytes.is_empty() {
    return Ok(Vec::new());
  }

  let mut items = Vec::new();
  let mut offset = 0;

  // Read item count
  if bytes.len() < offset + 8 {
    return Err(BatchingError::SerializationError(
      "Invalid batch: missing item count".to_string(),
    ));
  }

  let item_count = u64::from_le_bytes([
    bytes[offset],
    bytes[offset + 1],
    bytes[offset + 2],
    bytes[offset + 3],
    bytes[offset + 4],
    bytes[offset + 5],
    bytes[offset + 6],
    bytes[offset + 7],
  ]) as usize;
  offset += 8;

  // Read each item
  for _ in 0..item_count {
    // Read item length
    if bytes.len() < offset + 8 {
      return Err(BatchingError::SerializationError(format!(
        "Invalid batch: missing item length at offset {}",
        offset
      )));
    }

    let item_len = u64::from_le_bytes([
      bytes[offset],
      bytes[offset + 1],
      bytes[offset + 2],
      bytes[offset + 3],
      bytes[offset + 4],
      bytes[offset + 5],
      bytes[offset + 6],
      bytes[offset + 7],
    ]) as usize;
    offset += 8;

    // Read item data
    if bytes.len() < offset + item_len {
      return Err(BatchingError::SerializationError(format!(
        "Invalid batch: item data truncated at offset {}, expected {} bytes",
        offset, item_len
      )));
    }

    let item = Bytes::copy_from_slice(&bytes[offset..offset + item_len]);
    offset += item_len;
    items.push(item);
  }

  // Verify we consumed all bytes
  if offset != bytes.len() {
    return Err(BatchingError::SerializationError(format!(
      "Invalid batch: {} bytes remaining after deserialization",
      bytes.len() - offset
    )));
  }

  Ok(items)
}

/// A channel wrapper that adds batching logic to a regular channel.
///
/// This wrapper buffers items and sends them as batches when:
/// - The batch size threshold is reached, OR
/// - The timeout expires
///
/// Items are serialized as a batch before sending.
pub struct BatchingChannel {
  /// Inner channel sender
  inner: crate::channels::TypeErasedSender,
  /// Batch buffer (protected by async mutex for concurrent access)
  buffer: Arc<tokio::sync::Mutex<BatchBuffer>>,
  /// Batch configuration
  #[allow(dead_code)]
  config: BatchConfig,
  /// Background flush task handle
  flush_task: Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
  /// Shutdown signal
  shutdown: Arc<tokio::sync::Notify>,
}

impl BatchingChannel {
  /// Create a new `BatchingChannel` that wraps the given sender.
  ///
  /// # Arguments
  ///
  /// * `sender` - The underlying channel sender to wrap
  /// * `config` - Batch configuration
  ///
  /// # Returns
  ///
  /// A new `BatchingChannel` instance with a background flush task started.
  pub fn new(sender: crate::channels::TypeErasedSender, config: BatchConfig) -> Self {
    let buffer = Arc::new(tokio::sync::Mutex::new(BatchBuffer::new(config.clone())));
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let flush_task = Arc::new(tokio::sync::Mutex::new(None));

    let channel = Self {
      inner: sender.clone(),
      buffer: buffer.clone(),
      config: config.clone(),
      flush_task: flush_task.clone(),
      shutdown: shutdown.clone(),
    };

    // Start background flush task
    let task_handle = tokio::spawn({
      let buffer = buffer.clone();
      let config = config.clone();
      let shutdown = shutdown.clone();
      let sender = sender.clone();
      async move {
        let timeout = Duration::from_millis(config.batch_timeout_ms);
        loop {
          tokio::select! {
            _ = tokio::time::sleep(timeout) => {
              // Timeout expired, check if we should flush
              let mut buffer_guard = buffer.lock().await;
              if buffer_guard.should_flush() {
                let items = buffer_guard.flush();
                if !items.is_empty() {
                  // Serialize and send batch
                  if let Ok(batch_bytes) = serialize_batch(items) {
                    let _ = sender.send(ChannelItem::Bytes(batch_bytes)).await;
                  }
                }
              }
            }
            _ = shutdown.notified() => {
              // Shutdown signal received - flush remaining items
              let mut buffer_guard = buffer.lock().await;
              let items = buffer_guard.force_flush();
              if !items.is_empty()
                && let Ok(batch_bytes) = serialize_batch(items)
              {
                let _ = sender.send(ChannelItem::Bytes(batch_bytes)).await;
              }
              break;
            }
          }
        }
      }
    });

    // Store the task handle - we can't await in a non-async function,
    // so we use block_on to set it synchronously
    if let Ok(_handle) = tokio::runtime::Handle::try_current() {
      // If we're in a runtime, we can't use block_on from within an async context.
      // Spawn a blocking task in a separate thread to avoid the "runtime within runtime" error
      let task_handle_clone = task_handle;
      let flush_task_clone = flush_task.clone();
      std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
          .enable_all()
          .build()
          .unwrap();
        rt.block_on(async {
          *flush_task_clone.lock().await = Some(task_handle_clone);
        });
      })
      .join()
      .unwrap();
    } else {
      // If we're not in a tokio runtime, create a temporary runtime
      let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
      rt.block_on(async {
        *flush_task.lock().await = Some(task_handle);
      });
    }

    channel
  }

  /// Send an item through the batching channel.
  ///
  /// The item will be buffered and sent as part of a batch when conditions are met.
  ///
  /// # Arguments
  ///
  /// * `item` - The item to send (should be `ChannelItem::Bytes` for batching)
  ///
  /// # Returns
  ///
  /// `Ok(())` if the item was queued successfully, `Err` otherwise.
  pub async fn send(
    &self,
    item: crate::channels::ChannelItem,
  ) -> Result<(), crate::execution::ExecutionError> {
    // Extract Bytes from ChannelItem
    let bytes = match item {
      crate::channels::ChannelItem::Bytes(b) => b,
      _ => {
        // For non-Bytes items, send directly without batching
        return self.inner.send(item).await.map_err(|e| {
          crate::execution::ExecutionError::ChannelError {
            node: "unknown".to_string(),
            port: "unknown".to_string(),
            is_input: false,
            reason: format!("Failed to send item: {}", e),
          }
        });
      }
    };

    // Add to buffer
    let should_flush = {
      let mut buffer = self.buffer.lock().await;
      buffer.add(bytes).map_err(|e| {
        crate::execution::ExecutionError::ExecutionFailed(format!(
          "Failed to add item to batch buffer: {}",
          e
        ))
      })?;
      buffer.should_flush()
    };

    // Flush if needed
    if should_flush {
      self.flush().await?;
    }

    Ok(())
  }

  /// Flush the buffer, sending any pending items as a batch.
  ///
  /// # Returns
  ///
  /// `Ok(())` if the batch was sent successfully, `Err` otherwise.
  pub async fn flush(&self) -> Result<(), crate::execution::ExecutionError> {
    let items = {
      let mut buffer = self.buffer.lock().await;
      buffer.flush()
    };

    if items.is_empty() {
      return Ok(());
    }

    // Serialize batch
    let batch_bytes =
      serialize_batch(items).map_err(|e| crate::execution::ExecutionError::SerializationError {
        node: "unknown".to_string(),
        is_deserialization: false,
        reason: format!("Failed to serialize batch: {}", e),
      })?;

    // Send batch as Bytes
    self
      .inner
      .send(crate::channels::ChannelItem::Bytes(batch_bytes))
      .await
      .map_err(|e| crate::execution::ExecutionError::ChannelError {
        node: "unknown".to_string(),
        port: "unknown".to_string(),
        is_input: false,
        reason: format!("Failed to send batch: {}", e),
      })?;

    Ok(())
  }

  /// Shutdown the batching channel, flushing any remaining items.
  ///
  /// This should be called during graceful shutdown to ensure no items are lost.
  ///
  /// # Returns
  ///
  /// `Ok(())` if shutdown completed successfully, `Err` otherwise.
  pub async fn shutdown(&self) -> Result<(), crate::execution::ExecutionError> {
    // Signal shutdown to background task
    self.shutdown.notify_one();

    // Wait for background task to complete
    if let Some(handle) = self.flush_task.lock().await.take() {
      let _ = handle.await;
    }

    // Flush any remaining items
    self.flush().await?;

    Ok(())
  }

  /// Get a reference to the underlying channel sender.
  ///
  /// This can be used to send items directly without batching if needed.
  pub fn inner(&self) -> &crate::channels::TypeErasedSender {
    &self.inner
  }
}

impl Drop for BatchingChannel {
  fn drop(&mut self) {
    // Signal shutdown
    self.shutdown.notify_one();
  }
}
