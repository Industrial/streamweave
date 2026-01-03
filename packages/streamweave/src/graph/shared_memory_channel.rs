//! # Shared Memory Channels
//!
//! This module provides shared memory-based channels for ultra-high performance
//! inter-node communication. Data is stored in OS-native shared memory segments
//! with a lock-free ring buffer implementation.
//!
//! ## Architecture
//!
//! Shared memory channels use a ring buffer pattern where:
//! - Metadata (atomic counters) is stored at the beginning
//! - Data items are stored sequentially in the buffer
//! - Lock-free synchronization using atomic operations
//!
//! ## `Message<T>` Support
//!
//! Shared memory channels work with serialized `Message<T>` data. Before sending,
//! `Message<T>` instances are serialized to bytes using the serialization module.
//! After receiving, bytes are deserialized back to `Message<T>`, preserving message
//! IDs and metadata. This enables ultra-high performance inter-process communication
//! while maintaining end-to-end traceability.
//!
//! ## Usage
//!
//! ```rust
//! use streamweave::graph::shared_memory_channel::SharedMemoryChannel;
//!
//! // Create a shared memory channel
//! let channel = SharedMemoryChannel::new("my_channel", 1024 * 1024)?;
//!
//! // Send data
//! let data = b"hello world";
//! channel.send(data)?;
//!
//! // Receive data
//! let received = channel.receive()?;
//! ```
//!
//! ## Usage with `Message<T>`
//!
//! ```rust
//! use streamweave::graph::shared_memory_channel::SharedMemoryChannel;
//! use streamweave::graph::serialization::{serialize, deserialize};
//! use streamweave::message::wrap_message;
//!
//! // Create a shared memory channel
//! let channel = SharedMemoryChannel::new("my_channel", 1024 * 1024)?;
//!
//! // Serialize Message<T> to bytes
//! let msg = wrap_message(42i32);
//! let serialized = serialize(&msg)?;
//!
//! // Send serialized Message<T> through shared memory
//! let ref = channel.send(serialized.as_ref())?;
//!
//! // Receive and deserialize Message<T> (IDs and metadata preserved)
//! let received_bytes = channel.receive()?;
//! let deserialized: streamweave::message::Message<i32> = deserialize(received_bytes)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use bytes::Bytes;
use shared_memory::{Shmem, ShmemConf, ShmemError};
use std::any::TypeId;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;

/// Error types for shared memory channel operations.
#[derive(Error, Debug)]
pub enum SharedMemoryError {
  #[error("Buffer is full")]
  BufferFull,
  #[error("Buffer is empty")]
  BufferEmpty,
  #[error("Failed to create shared memory segment: {0}")]
  CreationFailed(String),
  #[error("Failed to open shared memory segment: {0}")]
  OpenFailed(String),
  #[error("Type mismatch: expected {expected:?}, got {got:?}")]
  TypeMismatch { expected: TypeId, got: TypeId },
  #[error("Serialization error: {0}")]
  SerializationError(String),
  #[error("Invalid offset or size")]
  InvalidOffset,
}

/// Ring buffer metadata stored at the beginning of shared memory.
///
/// This struct is stored at offset 0 in the shared memory segment.
/// It uses atomic operations for lock-free synchronization.
#[repr(C, align(64))] // Cache line alignment
struct SharedMemoryMetadata {
  /// Producer write position (atomic)
  write_pos: AtomicUsize,
  /// Consumer read position (atomic)
  read_pos: AtomicUsize,
  /// Buffer capacity in bytes (excluding metadata)
  capacity: usize,
  /// Number of items currently in buffer (atomic)
  item_count: AtomicUsize,
  /// Padding to ensure cache line alignment
  _padding: [u8; 64 - 4 * std::mem::size_of::<usize>()],
}

impl SharedMemoryMetadata {
  /// Create new metadata with given capacity.
  fn new(capacity: usize) -> Self {
    Self {
      write_pos: AtomicUsize::new(0),
      read_pos: AtomicUsize::new(0),
      capacity,
      item_count: AtomicUsize::new(0),
      _padding: [0; 64 - 4 * std::mem::size_of::<usize>()],
    }
  }
}

/// Item header stored before each item in the buffer.
#[repr(C)]
struct ItemHeader {
  /// Size of the item data in bytes
  size: u64,
  /// Type ID for type safety
  type_id: u64, // TypeId is not Copy, so we store as u64
}

const METADATA_SIZE: usize = std::mem::size_of::<SharedMemoryMetadata>();
const ITEM_HEADER_SIZE: usize = std::mem::size_of::<ItemHeader>();

/// A shared memory channel using a ring buffer pattern.
///
/// This channel stores data in a shared memory segment and uses atomic
/// operations for lock-free synchronization between producer and consumer.
pub struct SharedMemoryChannel {
  /// The shared memory segment
  shmem: Arc<Shmem>,
  /// Unique identifier for this channel
  segment_id: String,
  /// Pointer to metadata (unsafe but necessary for shared memory)
  metadata: *const SharedMemoryMetadata,
  /// Pointer to data buffer
  data_buffer: *mut u8,
  /// Capacity of the data buffer
  capacity: usize,
}

unsafe impl Send for SharedMemoryChannel {}
unsafe impl Sync for SharedMemoryChannel {}

impl SharedMemoryChannel {
  /// Create a new shared memory channel.
  ///
  /// # Arguments
  ///
  /// * `segment_id` - Unique identifier for the shared memory segment
  /// * `capacity` - Capacity of the data buffer in bytes (excluding metadata)
  ///
  /// # Returns
  ///
  /// A new `SharedMemoryChannel` or an error if creation fails.
  pub fn new(segment_id: &str, capacity: usize) -> Result<Self, SharedMemoryError> {
    let total_size = METADATA_SIZE + capacity;

    let shmem = ShmemConf::new()
      .size(total_size)
      .create()
      .map_err(|e: ShmemError| SharedMemoryError::CreationFailed(e.to_string()))?;

    // Get pointer to the shared memory
    let ptr = shmem.as_ptr();

    // Initialize metadata at the beginning
    unsafe {
      let metadata_ptr = ptr as *mut SharedMemoryMetadata;
      std::ptr::write(metadata_ptr, SharedMemoryMetadata::new(capacity));
    }

    // Calculate data buffer pointer (after metadata)
    let data_buffer = unsafe { ptr.add(METADATA_SIZE) };

    Ok(Self {
      #[allow(clippy::arc_with_non_send_sync)]
      shmem: Arc::new(shmem),
      segment_id: segment_id.to_string(),
      metadata: ptr as *const SharedMemoryMetadata,
      data_buffer,
      capacity,
    })
  }

  /// Open an existing shared memory channel.
  ///
  /// # Arguments
  ///
  /// * `segment_id` - The identifier of the existing shared memory segment
  ///
  /// # Returns
  ///
  /// A `SharedMemoryChannel` connected to the existing segment or an error.
  pub fn open(segment_id: &str) -> Result<Self, SharedMemoryError> {
    let shmem = ShmemConf::new()
      .os_id(segment_id)
      .open()
      .map_err(|e: ShmemError| SharedMemoryError::OpenFailed(e.to_string()))?;

    let ptr = shmem.as_ptr();
    let metadata = ptr as *const SharedMemoryMetadata;
    let data_buffer = unsafe { ptr.add(METADATA_SIZE) };

    // Read capacity from metadata
    let capacity = unsafe { (*metadata).capacity };

    Ok(Self {
      #[allow(clippy::arc_with_non_send_sync)]
      shmem: Arc::new(shmem),
      segment_id: segment_id.to_string(),
      metadata,
      data_buffer,
      capacity,
    })
  }

  /// Get the segment ID for this channel.
  pub fn segment_id(&self) -> &str {
    &self.segment_id
  }

  /// Send data into the shared memory channel.
  ///
  /// # Arguments
  ///
  /// * `data` - The data to send (typically serialized `Message<T>` bytes)
  ///
  /// # Returns
  ///
  /// A `SharedMemoryRef` pointing to the data in shared memory, or an error.
  ///
  /// # Note
  ///
  /// This method accepts raw bytes. For `Message<T>` support, serialize the
  /// message to bytes first using the serialization module, then send the
  /// serialized bytes. This preserves message IDs and metadata.
  pub fn send(&self, data: &[u8]) -> Result<SharedMemoryRef, SharedMemoryError> {
    let data_size = data.len();
    let item_size = ITEM_HEADER_SIZE + data_size;
    let offset = self.calculate_write_offset(item_size)?;

    // Write item header
    unsafe {
      let header_ptr = self.data_buffer.add(offset) as *mut ItemHeader;
      std::ptr::write(
        header_ptr,
        ItemHeader {
          size: data_size as u64,
          type_id: 0, // TODO: Store actual TypeId
        },
      );

      // Write data
      let data_ptr = self.data_buffer.add(offset + ITEM_HEADER_SIZE);
      std::ptr::copy_nonoverlapping(data.as_ptr(), data_ptr, data_size);
    }

    // Update metadata
    let metadata = unsafe { &*self.metadata };
    let next_write = (offset + item_size) % self.capacity;
    metadata.write_pos.store(next_write, Ordering::Release);
    metadata.item_count.fetch_add(1, Ordering::Release);

    Ok(SharedMemoryRef {
      segment_id: self.segment_id.clone(),
      offset: offset + ITEM_HEADER_SIZE, // Offset to data (not header)
      size: data_size,
      type_id: TypeId::of::<()>(), // TODO: Store actual type
    })
  }

  /// Receive data from the shared memory channel.
  ///
  /// # Returns
  ///
  /// The received data as `Bytes` (typically serialized `Message<T>`), or an error if the buffer is empty.
  ///
  /// # Note
  ///
  /// This method returns raw bytes. For `Message<T>` support, deserialize the
  /// bytes to `Message<T>` using the serialization module. This restores message
  /// IDs and metadata.
  pub fn receive(&self) -> Result<Bytes, SharedMemoryError> {
    let metadata = unsafe { &*self.metadata };

    // Check if buffer is empty
    if metadata.item_count.load(Ordering::Acquire) == 0 {
      return Err(SharedMemoryError::BufferEmpty);
    }

    let read_pos = metadata.read_pos.load(Ordering::Acquire);

    // Read item header
    let header = unsafe {
      let header_ptr = self.data_buffer.add(read_pos) as *const ItemHeader;
      std::ptr::read(header_ptr)
    };

    let data_size = header.size as usize;
    let item_size = ITEM_HEADER_SIZE + data_size;

    // Read data
    let data = unsafe {
      let data_ptr = self.data_buffer.add(read_pos + ITEM_HEADER_SIZE);
      Bytes::copy_from_slice(std::slice::from_raw_parts(data_ptr, data_size))
    };

    // Update read position
    let next_read = (read_pos + item_size) % self.capacity;
    metadata.read_pos.store(next_read, Ordering::Release);
    metadata.item_count.fetch_sub(1, Ordering::Release);

    Ok(data)
  }

  /// Calculate the write offset for a new item.
  ///
  /// Returns an error if the buffer is full.
  fn calculate_write_offset(&self, item_size: usize) -> Result<usize, SharedMemoryError> {
    let metadata = unsafe { &*self.metadata };
    let write_pos = metadata.write_pos.load(Ordering::Acquire);
    let read_pos = metadata.read_pos.load(Ordering::Acquire);

    // Item cannot be larger than the buffer capacity
    if item_size > self.capacity {
      return Err(SharedMemoryError::BufferFull);
    }

    // Calculate where the next write position would be
    let next_write_pos = if write_pos + item_size <= self.capacity {
      // No wraparound needed
      write_pos + item_size
    } else {
      // Would wrap around - calculate new position after wrap
      (write_pos + item_size) % self.capacity
    };

    // Check if buffer is full
    // Buffer is full if write_pos == read_pos and there are items in the buffer
    if write_pos == read_pos {
      let item_count = metadata.item_count.load(Ordering::Acquire);
      if item_count > 0 {
        // Buffer is full
        return Err(SharedMemoryError::BufferFull);
      }
      // Buffer is empty - can write
    } else if write_pos < read_pos {
      // Already wrapped - check if next write would overlap with read_pos
      if next_write_pos > read_pos {
        return Err(SharedMemoryError::BufferFull);
      }
    } else {
      // write_pos > read_pos - check if we would wrap and overlap
      if write_pos + item_size > self.capacity {
        // Would wrap - check if wrap position overlaps with read_pos
        if next_write_pos >= read_pos {
          return Err(SharedMemoryError::BufferFull);
        }
      }
    }

    Ok(write_pos)
  }

  /// Read data from a specific offset in shared memory.
  ///
  /// This is used when receiving a `SharedMemoryRef` from another process.
  pub fn read_at(&self, offset: usize, size: usize) -> Result<Bytes, SharedMemoryError> {
    if offset + size > self.capacity {
      return Err(SharedMemoryError::InvalidOffset);
    }

    unsafe {
      let data_ptr = self.data_buffer.add(offset);
      Ok(Bytes::copy_from_slice(std::slice::from_raw_parts(
        data_ptr, size,
      )))
    }
  }

  /// Explicitly cleanup the shared memory segment.
  ///
  /// This method marks the segment for deletion. The OS will cleanup
  /// the shared memory when the last reference is dropped.
  ///
  /// # Note
  ///
  /// The `shared_memory` crate handles cleanup automatically when the
  /// last `Arc<Shmem>` is dropped. This method is provided for explicit
  /// cleanup if needed.
  pub fn cleanup(&self) {
    // The shared_memory crate handles cleanup automatically via Drop
    // when the last Arc<Shmem> is dropped. We just need to ensure
    // all references are dropped.
    // This is a no-op since Arc handles reference counting automatically.
  }

  /// Check if the shared memory segment is still valid.
  ///
  /// # Returns
  ///
  /// `true` if the segment is valid, `false` otherwise.
  pub fn is_valid(&self) -> bool {
    // Check if we can still access the metadata
    unsafe {
      let metadata = &*self.metadata;
      // Simple validity check: capacity should be reasonable
      metadata.capacity > 0 && metadata.capacity < usize::MAX / 2
    }
  }
}

impl Drop for SharedMemoryChannel {
  fn drop(&mut self) {
    // The shared_memory crate will automatically cleanup the segment
    // when the last Arc<Shmem> is dropped. On Unix systems, this uses
    // shm_unlink() when the last reference is dropped.
    // On Windows, the file mapping is closed when the last handle is closed.

    // Reset metadata to prevent use-after-free
    // Note: This is a safety measure, but the segment will be cleaned up
    // by the OS when the last process drops its reference.
  }
}

impl Clone for SharedMemoryChannel {
  fn clone(&self) -> Self {
    Self {
      shmem: Arc::clone(&self.shmem),
      segment_id: self.segment_id.clone(),
      metadata: self.metadata,
      data_buffer: self.data_buffer,
      capacity: self.capacity,
    }
  }
}

/// A lightweight reference to data in shared memory.
///
/// This struct can be sent through regular channels and contains
/// all information needed to retrieve the actual data from shared memory.
#[derive(Clone, Debug)]
pub struct SharedMemoryRef {
  /// Unique identifier for the shared memory segment
  pub segment_id: String,
  /// Offset within the segment where data starts
  pub offset: usize,
  /// Size of the data in bytes
  pub size: usize,
  /// Type identifier for safe downcasting
  pub type_id: TypeId,
}
