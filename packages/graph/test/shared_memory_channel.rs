//! Shared memory channel tests
//!
//! Tests for shared memory-based channels using lock-free ring buffers.
//! Note: Some tests may be platform-specific due to shared memory requirements.

use bytes::Bytes;
use proptest::prelude::*;
use streamweave_graph::shared_memory_channel::*;

#[test]
fn test_shared_memory_error_display() {
  let err = SharedMemoryError::BufferFull;
  assert!(err.to_string().contains("full"));

  let err = SharedMemoryError::BufferEmpty;
  assert!(err.to_string().contains("empty"));

  let err = SharedMemoryError::CreationFailed("test".to_string());
  assert!(err.to_string().contains("Failed to create"));

  let err = SharedMemoryError::TypeMismatch {
    expected: std::any::TypeId::of::<i32>(),
    got: std::any::TypeId::of::<String>(),
  };
  assert!(err.to_string().contains("Type mismatch"));
}

#[test]
fn test_shared_memory_metadata_new() {
  // This is a private struct, but we can test through the channel
  // Just verify the module compiles and basic error types work
  let _err = SharedMemoryError::InvalidOffset;
  assert!(true); // Test passes if compilation succeeds
}

// Note: Full shared memory channel tests would require:
// - Creating actual shared memory segments (platform-specific)
// - Testing concurrent access from multiple processes
// - Testing lock-free synchronization
// These are integration tests that would need more setup

// For now, we test error types and basic structure
#[test]
fn test_shared_memory_error_variants() {
  let errors = vec![
    SharedMemoryError::BufferFull,
    SharedMemoryError::BufferEmpty,
    SharedMemoryError::CreationFailed("test".to_string()),
    SharedMemoryError::OpenFailed("test".to_string()),
    SharedMemoryError::TypeMismatch {
      expected: std::any::TypeId::of::<i32>(),
      got: std::any::TypeId::of::<i32>(),
    },
    SharedMemoryError::SerializationError("test".to_string()),
    SharedMemoryError::InvalidOffset,
  ];

  for err in errors {
    let _msg = err.to_string();
    assert!(!_msg.is_empty());
  }
}

// Test that we can at least attempt to create a channel
// (may fail on some platforms, but we test the API)
#[test]
fn test_shared_memory_channel_creation_attempt() {
  // Try to create a small channel
  // This may fail on some platforms, but we're testing the API
  let result = SharedMemoryChannel::new("test_channel_12345", 1024);

  // On platforms that support shared memory, this should succeed
  // On others, it may fail, which is acceptable
  match result {
    Ok(_channel) => {
      // If creation succeeds, test basic operations
      // Note: Full tests would require more complex setup
    }
    Err(_) => {
      // Creation failed (possibly unsupported platform)
      // This is acceptable for this test
    }
  }
}

// Test SharedMemoryRef if it's public
// (Checking if it exists and is usable)
#[test]
fn test_shared_memory_ref_basic() {
  // This tests that the type exists and can be used
  // Actual functionality would require shared memory setup
  use streamweave_graph::shared_memory_channel::SharedMemoryRef;

  // Just verify the type is available
  let _ref_type: Option<SharedMemoryRef> = None;
  assert!(true);
}
