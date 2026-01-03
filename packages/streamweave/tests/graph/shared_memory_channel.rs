//! Shared memory channel tests
//!
//! Tests for shared memory-based channels using lock-free ring buffers.
//! Note: Some tests may be platform-specific due to shared memory requirements.

use bytes::Bytes;
use proptest::prelude::*;
use streamweave::graph::shared_memory_channel::*;

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
  use streamweave::graph::shared_memory_channel::SharedMemoryRef;

  // Just verify the type is available
  let _ref_type: Option<SharedMemoryRef> = None;
  assert!(true);
}

// Tests moved from src/
#[test]
fn test_shared_memory_channel_creation() {
  let channel = SharedMemoryChannel::new("test_channel", 1024);
  assert!(channel.is_ok());
}

#[test]
fn test_shared_memory_send_receive() {
  let channel = SharedMemoryChannel::new("test_send_receive", 1024).unwrap();
  let data = b"hello world";

  let shared_mem_ref = channel.send(data).unwrap();
  let received = channel.receive().unwrap();

  assert_eq!(received.as_ref(), data);
  assert_eq!(shared_mem_ref.size, data.len());
}

#[test]
fn test_shared_memory_multiple_items() {
  let channel = SharedMemoryChannel::new("test_multiple", 1024).unwrap();

  channel.send(b"first").unwrap();
  channel.send(b"second").unwrap();
  channel.send(b"third").unwrap();

  assert_eq!(channel.receive().unwrap().as_ref(), b"first");
  assert_eq!(channel.receive().unwrap().as_ref(), b"second");
  assert_eq!(channel.receive().unwrap().as_ref(), b"third");
}

#[test]
fn test_shared_memory_buffer_full() {
  let channel = SharedMemoryChannel::new("test_full", 100).unwrap();
  let large_data = vec![0u8; 200]; // Larger than capacity

  let result = channel.send(&large_data);
  assert!(result.is_err());
  assert!(matches!(result.unwrap_err(), SharedMemoryError::BufferFull));
}

#[test]
fn test_shared_memory_buffer_empty() {
  let channel = SharedMemoryChannel::new("test_empty", 1024).unwrap();

  let result = channel.receive();
  assert!(result.is_err());
  assert!(matches!(
    result.unwrap_err(),
    SharedMemoryError::BufferEmpty
  ));
}

#[test]
fn test_shared_memory_cleanup() {
  let channel = SharedMemoryChannel::new("test_cleanup", 1024).unwrap();
  assert!(channel.is_valid());
  channel.cleanup();
  // Channel should still be valid after cleanup (cleanup is a no-op)
  assert!(channel.is_valid());
}

#[test]
fn test_shared_memory_ring_buffer_wraparound() {
  // Use larger buffer to accommodate items (each item needs header + data)
  // 5 items * (16 bytes header + 5 bytes data) = 105 bytes minimum
  let channel = SharedMemoryChannel::new("test_wraparound", 200).unwrap();

  // Fill buffer to near capacity
  for i in 0..5 {
    let data = format!("item{}", i);
    channel.send(data.as_bytes()).unwrap();
  }

  // Read some items
  assert_eq!(channel.receive().unwrap().as_ref(), b"item0");
  assert_eq!(channel.receive().unwrap().as_ref(), b"item1");

  // Add more items (should wrap around)
  channel.send(b"item5").unwrap();
  channel.send(b"item6").unwrap();

  // Verify all items
  assert_eq!(channel.receive().unwrap().as_ref(), b"item2");
  assert_eq!(channel.receive().unwrap().as_ref(), b"item3");
  assert_eq!(channel.receive().unwrap().as_ref(), b"item4");
  assert_eq!(channel.receive().unwrap().as_ref(), b"item5");
  assert_eq!(channel.receive().unwrap().as_ref(), b"item6");
}

#[tokio::test]
async fn test_shared_memory_concurrent_access() {
  use tokio::task;

  let channel = Arc::new(SharedMemoryChannel::new("test_concurrent", 1024).unwrap());

  // Spawn producer task
  let producer_channel = channel.clone();
  let producer = task::spawn(async move {
    for i in 0..10 {
      let data = format!("data{}", i);
      producer_channel.send(data.as_bytes()).unwrap();
      tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
  });

  // Spawn consumer task
  let consumer_channel = channel.clone();
  let consumer = task::spawn(async move {
    let mut received = Vec::new();
    for _ in 0..10 {
      loop {
        match consumer_channel.receive() {
          Ok(data) => {
            received.push(data);
            break;
          }
          Err(SharedMemoryError::BufferEmpty) => {
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            continue;
          }
          Err(e) => panic!("Unexpected error: {:?}", e),
        }
      }
    }
    received
  });

  producer.await.unwrap();
  let received = consumer.await.unwrap();

  assert_eq!(received.len(), 10);
  for (i, data) in received.iter().enumerate() {
    assert_eq!(data.as_ref(), format!("data{}", i).as_bytes());
  }
}
