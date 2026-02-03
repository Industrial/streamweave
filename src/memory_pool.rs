//! # Memory Pooling System
//!
//! Provides efficient memory reuse for high-performance streaming operations.
//! Implements buffer pooling and string interning to reduce allocation overhead.
//!
//! ## Overview
//!
//! This module provides:
//!
//! - **Buffer Pooling**: Reusable `BytesMut` buffers for serialization/deserialization
//! - **String Interning**: Shared string storage to eliminate duplicate allocations
//! - **Memory Management**: Automatic cleanup and size limits
//! - **Thread Safety**: Lock-free operations where possible
//!
//! ## Performance Benefits
//!
//! - **Reduced GC Pressure**: Reuse buffers instead of frequent allocations
//! - **Lower Latency**: Avoid malloc overhead in hot paths
//! - **Better Cache Locality**: Reuse memory reduces cache thrashing
//! - **Memory Efficiency**: String interning eliminates duplicates
//!
//! ## Usage
//!
//! ```rust,no_run
//! use streamweave::memory_pool::{MemoryPool, StringInterner};
//!
//! // Get a pooled buffer for serialization
//! let mut buffer = MemoryPool::get_buffer(1024);
//! serde_json::to_writer(&mut buffer, &data)?;
//! let bytes = buffer.freeze();
//!
//! // Intern frequently-used strings
//! let key = StringInterner::intern("configuration");
//! ```

use bytes::BytesMut;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Global memory pool instance for buffer management.
static BUFFER_POOL: once_cell::sync::Lazy<Arc<MemoryPool>> =
  once_cell::sync::Lazy::new(|| Arc::new(MemoryPool::new()));

/// Global string interner instance.
static STRING_INTERNER: once_cell::sync::Lazy<Arc<StringInterner>> =
  once_cell::sync::Lazy::new(|| Arc::new(StringInterner::new()));

/// Configuration for memory pool sizing.
#[derive(Debug, Clone)]
pub struct MemoryPoolConfig {
  /// Maximum number of buffers to keep in pool per size class
  pub max_buffers_per_size: usize,
  /// Size classes for buffer pooling (powers of 2)
  pub size_classes: Vec<usize>,
  /// Maximum size for string interning (to prevent unbounded growth)
  pub max_interned_strings: usize,
}

impl Default for MemoryPoolConfig {
  fn default() -> Self {
    Self {
      max_buffers_per_size: 1000,
      size_classes: vec![
        64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536,
      ],
      max_interned_strings: 10000,
    }
  }
}

/// Thread-safe memory pool for buffer reuse.
///
/// This pool maintains separate pools for different buffer sizes to optimize
/// memory usage and reduce allocation overhead.
pub struct MemoryPool {
  /// Map of buffer size to available buffers in that size pool.
  pools: Mutex<HashMap<usize, Vec<BytesMut>>>,
  /// Configuration settings for the memory pool.
  config: MemoryPoolConfig,
}

impl Default for MemoryPool {
  fn default() -> Self {
    Self::new()
  }
}

impl MemoryPool {
  /// Creates a new memory pool with default configuration.
  pub fn new() -> Self {
    Self::with_config(MemoryPoolConfig::default())
  }

  /// Creates a new memory pool with custom configuration.
  pub fn with_config(config: MemoryPoolConfig) -> Self {
    Self {
      pools: Mutex::new(HashMap::new()),
      config,
    }
  }

  /// Gets the global memory pool instance.
  pub fn global() -> &'static Arc<MemoryPool> {
    &BUFFER_POOL
  }

  /// Gets a buffer from the pool with at least the requested capacity.
  ///
  /// Returns a buffer that can hold at least `capacity` bytes. The actual
  /// buffer may be larger to fit size class requirements.
  pub async fn get_buffer(&self, capacity: usize) -> BytesMut {
    let size_class = self.find_size_class(capacity);
    let mut pools = self.pools.lock().await;

    // Try to get a buffer from the appropriate size class
    if let Some(buffers) = pools.get_mut(&size_class)
      && let Some(buffer) = buffers.pop()
    {
      // Clear the buffer but keep its capacity
      let mut buffer = buffer;
      buffer.clear();
      // Ensure it has enough capacity
      if buffer.capacity() < capacity {
        buffer.reserve(capacity - buffer.capacity());
      }
      return buffer;
    }

    // No pooled buffer available, create a new one
    BytesMut::with_capacity(size_class)
  }

  /// Returns a buffer to the pool for reuse.
  ///
  /// The buffer will be cleared and stored in the appropriate size class
  /// pool if there's space available.
  pub async fn return_buffer(&self, mut buffer: BytesMut) {
    let size_class = self.find_size_class(buffer.capacity());
    let mut pools = self.pools.lock().await;

    let pool = pools.entry(size_class).or_insert_with(Vec::new);

    // Only keep the buffer if we haven't exceeded the limit
    if pool.len() < self.config.max_buffers_per_size {
      buffer.clear();
      pool.push(buffer);
    }
    // If pool is full, buffer will be dropped (memory freed)
  }

  /// Finds the smallest size class that can accommodate the requested capacity.
  fn find_size_class(&self, capacity: usize) -> usize {
    for &size in &self.config.size_classes {
      if size >= capacity {
        return size;
      }
    }
    // If capacity is larger than all size classes, round up to next power of 2
    capacity.next_power_of_two().max(64)
  }

  /// Gets pool statistics for monitoring and debugging.
  pub async fn stats(&self) -> MemoryPoolStats {
    let pools = self.pools.lock().await;
    let mut total_buffers = 0;
    let mut total_memory = 0;

    for (size_class, buffers) in &*pools {
      total_buffers += buffers.len();
      total_memory += buffers.len() * size_class;
    }

    MemoryPoolStats {
      total_buffers,
      total_memory_bytes: total_memory,
      pools_count: pools.len(),
    }
  }
}

/// Statistics about memory pool usage.
#[derive(Debug, Clone)]
pub struct MemoryPoolStats {
  /// Total number of buffers currently in all pools
  pub total_buffers: usize,
  /// Total memory used by pooled buffers in bytes
  pub total_memory_bytes: usize,
  /// Number of size class pools
  pub pools_count: usize,
}

/// Thread-safe string interning for efficient string reuse.
///
/// Strings are stored in a global registry and reused when the same string
/// is requested multiple times, eliminating duplicate allocations.
pub struct StringInterner {
  /// Map of string values to interned Arc<str> instances.
  strings: Mutex<HashMap<String, Arc<str>>>,
  /// Configuration settings for the string interner.
  config: MemoryPoolConfig,
}

impl Default for StringInterner {
  fn default() -> Self {
    Self::new()
  }
}

impl StringInterner {
  /// Creates a new string interner with default configuration.
  pub fn new() -> Self {
    Self::with_config(MemoryPoolConfig::default())
  }

  /// Creates a new string interner with custom configuration.
  pub fn with_config(config: MemoryPoolConfig) -> Self {
    Self {
      strings: Mutex::new(HashMap::new()),
      config,
    }
  }

  /// Gets the global string interner instance.
  pub fn global() -> &'static Arc<StringInterner> {
    &STRING_INTERNER
  }

  /// Interns a string, returning a shared reference.
  ///
  /// If the string is already interned, returns the existing reference.
  /// Otherwise, stores the string and returns a reference to it.
  pub async fn intern(&self, s: &str) -> Arc<str> {
    let mut strings = self.strings.lock().await;

    if let Some(interned) = strings.get(s) {
      return Arc::clone(interned);
    }

    // Check if we've exceeded the limit and need to evict
    if strings.len() >= self.config.max_interned_strings {
      // Simple eviction: remove oldest entries (this could be improved)
      let to_remove: Vec<String> = strings
        .keys()
        .take(strings.len() - self.config.max_interned_strings + 100) // Keep some margin
        .cloned()
        .collect();
      for key in to_remove {
        strings.remove(&key);
      }
    }

    let interned: Arc<str> = Arc::from(s);
    strings.insert(s.to_string(), Arc::clone(&interned));
    interned
  }

  /// Interns a string synchronously (for cases where async is not available).
  ///
  /// This method uses a blocking mutex and should only be used in synchronous contexts
  /// or when performance is not critical.
  pub fn intern_sync(&self, s: &str) -> Arc<str> {
    // This is a simplified sync version - in production, you'd want a proper sync implementation
    futures::executor::block_on(self.intern(s))
  }

  /// Gets interner statistics for monitoring.
  pub async fn stats(&self) -> StringInternerStats {
    let strings = self.strings.lock().await;
    StringInternerStats {
      total_strings: strings.len(),
      total_memory_bytes: strings.values().map(|s| s.len()).sum(),
    }
  }
}

/// Statistics about string interner usage.
#[derive(Debug, Clone)]
pub struct StringInternerStats {
  /// Total number of unique strings interned
  pub total_strings: usize,
  /// Total memory used by interned strings in bytes
  pub total_memory_bytes: usize,
}

/// Convenience functions for global memory pool access.
///
/// These functions provide easy access to the global memory pool and string interner
/// without having to manually access the static instances.
pub mod global {
  use super::*;

  /// Gets a buffer from the global memory pool.
  pub async fn get_buffer(capacity: usize) -> BytesMut {
    MemoryPool::global().get_buffer(capacity).await
  }

  /// Returns a buffer to the global memory pool.
  pub async fn return_buffer(buffer: BytesMut) {
    MemoryPool::global().return_buffer(buffer).await
  }

  /// Interns a string using the global string interner.
  pub async fn intern_string(s: &str) -> Arc<str> {
    StringInterner::global().intern(s).await
  }

  /// Interns a string synchronously using the global string interner.
  pub fn intern_string_sync(s: &str) -> Arc<str> {
    StringInterner::global().intern_sync(s)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_buffer_pooling() {
    let pool = MemoryPool::new();

    // Get a buffer
    let mut buffer = pool.get_buffer(100).await;
    assert!(buffer.capacity() >= 100);

    // Use the buffer
    buffer.extend_from_slice(b"hello world");
    assert_eq!(&buffer[..], b"hello world");

    // Return the buffer
    pool.return_buffer(buffer).await;

    // Get another buffer (should reuse)
    let buffer2 = pool.get_buffer(100).await;
    assert!(buffer2.capacity() >= 100); // Should have same or larger capacity

    // Verify pool stats
    let stats = pool.stats().await;
    assert_eq!(stats.total_buffers, 0); // Buffer was reused from pool
  }

  #[tokio::test]
  async fn test_string_interning() {
    let interner = StringInterner::new();

    // Intern a string
    let s1 = interner.intern("hello").await;
    let s2 = interner.intern("hello").await;
    let s3 = interner.intern("world").await;

    // Same strings should return the same reference
    assert!(Arc::ptr_eq(&s1, &s2));
    assert!(!Arc::ptr_eq(&s1, &s3));

    // Values should be correct
    assert_eq!(&*s1, "hello");
    assert_eq!(&*s3, "world");

    // Verify stats
    let stats = interner.stats().await;
    assert_eq!(stats.total_strings, 2);
    assert!(stats.total_memory_bytes >= 10); // "hello" + "world"
  }

  #[tokio::test]
  async fn test_global_access() {
    // Test global buffer access
    let buffer = global::get_buffer(64).await;
    assert!(buffer.capacity() >= 64);

    // Test global string interning
    let s1 = global::intern_string("test").await;
    let s2 = global::intern_string("test").await;
    assert!(Arc::ptr_eq(&s1, &s2));
  }

  #[tokio::test]
  async fn test_size_classes() {
    let pool = MemoryPool::new();

    // Test various sizes map to appropriate size classes
    let buffer64 = pool.get_buffer(32).await; // Should get 64 class
    let buffer128 = pool.get_buffer(100).await; // Should get 128 class

    assert!(buffer64.capacity() >= 64);
    assert!(buffer128.capacity() >= 128);
  }

  #[tokio::test]
  async fn test_pool_limits() {
    let config = MemoryPoolConfig {
      max_buffers_per_size: 2,
      ..Default::default()
    };
    let pool = MemoryPool::with_config(config);

    // Fill the pool
    let b1 = pool.get_buffer(64).await;
    let b2 = pool.get_buffer(64).await;
    let b3 = pool.get_buffer(64).await; // This will create a new buffer

    // Return buffers (only 2 should be kept)
    pool.return_buffer(b1).await;
    pool.return_buffer(b2).await;
    pool.return_buffer(b3).await; // This should be dropped

    let stats = pool.stats().await;
    assert_eq!(stats.total_buffers, 2); // Only 2 buffers kept
  }
}
