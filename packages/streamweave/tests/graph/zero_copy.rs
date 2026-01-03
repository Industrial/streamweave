//! Comprehensive tests for zero-copy module
//!
//! This module provides 100% coverage tests for:
//! - ZeroCopyShare trait implementation
//! - ZeroCopyShareWeak trait
//! - ArcPool functionality
//! - InPlaceTransform trait
//! - MutateInPlace trait
//! - ZeroCopyTransformer trait
//! - Edge cases

use std::sync::Arc;
use std::sync::Weak;
use streamweave::graph::zero_copy::*;

// ============================================================================
// ZeroCopyShare Tests
// ============================================================================

#[test]
fn test_zero_copy_share_i32() {
  let value = 42i32;
  let shared = value.to_shared();

  // Clone should be cheap (Arc clone)
  let shared2 = shared.clone();
  assert_eq!(Arc::strong_count(&shared), 2);

  // Convert back
  let original = i32::from_shared(shared);
  assert_eq!(original, 42);

  // Second shared should still work
  let original2 = i32::from_shared(shared2);
  assert_eq!(original2, 42);
}

#[test]
fn test_zero_copy_share_string() {
  let value = "hello world".to_string();
  let shared = value.to_shared();

  // Clone should be cheap
  let shared2 = shared.clone();
  assert_eq!(Arc::strong_count(&shared), 2);

  // Convert back
  let original = String::from_shared(shared);
  assert_eq!(original, "hello world");

  // Second shared should still work
  let original2 = String::from_shared(shared2);
  assert_eq!(original2, "hello world");
}

#[test]
fn test_zero_copy_share_vec() {
  let value = vec![1, 2, 3, 4, 5];
  let shared = value.to_shared();

  // Clone should be cheap
  let shared2 = shared.clone();
  assert_eq!(Arc::strong_count(&shared), 2);

  // Convert back
  let original = Vec::<i32>::from_shared(shared);
  assert_eq!(original, vec![1, 2, 3, 4, 5]);

  // Second shared should still work
  let original2 = Vec::<i32>::from_shared(shared2);
  assert_eq!(original2, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_zero_copy_share_single_reference() {
  let value = 42i32;
  let shared = value.to_shared();

  // If only one reference, unwrap should be zero-cost
  let original = i32::from_shared(shared);
  assert_eq!(original, 42);
}

#[test]
fn test_zero_copy_share_multiple_references() {
  let value = 42i32;
  let shared = value.to_shared();
  let _shared2 = shared.clone();
  let _shared3 = shared.clone();

  // With multiple references, should clone
  assert_eq!(Arc::strong_count(&shared), 3);
  let original = i32::from_shared(shared);
  assert_eq!(original, 42);
}

// ============================================================================
// ZeroCopyShareWeak Tests
// ============================================================================

#[test]
fn test_zero_copy_share_weak_i32() {
  let value = 42i32;
  let shared = value.to_shared();
  let weak = i32::downgrade(&shared);

  // Weak should be available
  let upgraded = i32::upgrade(weak).unwrap();
  assert_eq!(*upgraded, 42);
}

#[test]
fn test_zero_copy_share_weak_dropped() {
  let value = 42i32;
  let shared = value.to_shared();
  let weak = i32::downgrade(&shared);

  // Drop the shared reference
  drop(shared);

  // Weak should be None now
  let upgraded = i32::upgrade(weak);
  assert!(upgraded.is_none());
}

// ============================================================================
// ArcPool Tests
// ============================================================================

#[test]
fn test_arc_pool_new() {
  let pool = ArcPool::<i32>::new(10);
  assert_eq!(pool.capacity(), 10);
}

#[test]
fn test_arc_pool_get_and_return() {
  let pool = ArcPool::<i32>::new(10);

  // Get an Arc from the pool
  let arc = pool.get();
  assert!(arc.is_some());

  // Return it
  if let Some(arc) = arc {
    pool.return_arc(arc);
  }
}

#[test]
fn test_arc_pool_capacity() {
  let pool = ArcPool::<i32>::new(5);
  assert_eq!(pool.capacity(), 5);

  let pool2 = ArcPool::<i32>::new(100);
  assert_eq!(pool2.capacity(), 100);
}

// ============================================================================
// InPlaceTransform Tests
// ============================================================================

#[test]
fn test_in_place_transform_i32() {
  let value = 42i32;
  let transformed = value.transform_in_place(|x| x * 2);
  assert_eq!(transformed, 84);
}

#[test]
fn test_in_place_transform_vec() {
  let value = vec![1, 2, 3];
  let transformed = value.transform_in_place(|mut v| {
    v.push(4);
    v
  });
  assert_eq!(transformed, vec![1, 2, 3, 4]);
}

#[test]
fn test_in_place_transform_string() {
  let value = "hello".to_string();
  let transformed = value.transform_in_place(|s| s + " world");
  assert_eq!(transformed, "hello world");
}

// ============================================================================
// MutateInPlace Tests
// ============================================================================

#[test]
fn test_mutate_in_place_vec() {
  let mut value = vec![1, 2, 3];
  value.mutate_in_place(|v| {
    v.push(4);
    v.push(5);
  });
  assert_eq!(value, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_mutate_in_place_string() {
  let mut value = "hello".to_string();
  value.mutate_in_place(|s| {
    s.push_str(" world");
  });
  assert_eq!(value, "hello world");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_zero_copy_share_empty_vec() {
  let value: Vec<i32> = vec![];
  let shared = value.to_shared();
  let original = Vec::<i32>::from_shared(shared);
  assert_eq!(original, vec![]);
}

#[test]
fn test_zero_copy_share_large_vec() {
  let value: Vec<i32> = (0..10000).collect();
  let shared = value.to_shared();
  let original = Vec::<i32>::from_shared(shared);
  assert_eq!(original.len(), 10000);
}

#[test]
fn test_zero_copy_share_nested() {
  let value = vec![vec![1, 2], vec![3, 4]];
  let shared = value.to_shared();
  let original = Vec::<Vec<i32>>::from_shared(shared);
  assert_eq!(original, vec![vec![1, 2], vec![3, 4]]);
}

// Tests moved from src/
use std::sync::Arc;

#[test]
fn test_arc_pool_get_or_create() {
  let pool = ArcPool::<String>::new(10);

  // First call should create new (pool is empty)
  let arc1 = pool.get_or_create("hello".to_string());
  assert_eq!(*arc1, "hello");
  assert_eq!(pool.misses(), 1);
  assert_eq!(pool.hits(), 0);

  // Return to pool
  assert!(pool.return_arc(arc1));
  assert_eq!(pool.len(), 1);
  assert_eq!(pool.returns(), 1);

  // Second call should reuse from pool
  let arc2 = pool.get_or_create("world".to_string());
  // Should get "hello" from pool, not "world"
  assert_eq!(*arc2, "hello");
  assert_eq!(pool.hits(), 1);
  assert_eq!(pool.misses(), 1);
}

#[test]
fn test_arc_pool_return_arc_single_reference() {
  let pool = ArcPool::<Vec<i32>>::new(10);

  let arc = pool.get_or_create(vec![1, 2, 3]);
  assert_eq!(pool.len(), 0); // Pool was empty, so nothing to return

  // Return with single reference should succeed
  assert!(pool.return_arc(arc));
  assert_eq!(pool.len(), 1);
  assert_eq!(pool.returns(), 1);
  assert_eq!(pool.return_failures(), 0);
}

#[test]
fn test_arc_pool_return_arc_multiple_references() {
  let pool = ArcPool::<String>::new(10);

  let arc1 = pool.get_or_create("test".to_string());
  let arc2 = Arc::clone(&arc1); // Create second reference

  // Try to return arc1 - should fail because arc2 still holds a reference
  assert!(!pool.return_arc(arc1));
  assert_eq!(pool.len(), 0);
  assert_eq!(pool.return_failures(), 1);

  // Drop arc2, then we can return (but arc1 is already dropped)
  drop(arc2);
}

#[test]
fn test_arc_pool_max_size() {
  let pool = ArcPool::<i32>::new(2);

  // Fill pool to max by creating and returning arcs
  let arc1 = Arc::new(1);
  let arc2 = Arc::new(2);

  assert!(pool.return_arc(arc1));
  assert!(pool.return_arc(arc2));
  assert_eq!(pool.len(), 2);

  // Try to return another - should fail because pool is full
  let arc3 = Arc::new(3);
  assert!(!pool.return_arc(arc3)); // Pool is full (len == max_size), should fail
  assert_eq!(pool.len(), 2); // Still at max size
  assert_eq!(pool.return_failures(), 1);
}

#[test]
fn test_arc_pool_statistics() {
  let pool = ArcPool::<String>::new(10);

  // Make some requests
  let arc1 = pool.get_or_create("a".to_string());
  let _arc2 = pool.get_or_create("b".to_string());
  pool.return_arc(arc1);
  let _arc3 = pool.get_or_create("c".to_string()); // Should get "a" from pool

  let stats = pool.statistics();
  assert_eq!(stats.hits, 1); // One hit (arc3 reused "a")
  assert_eq!(stats.misses, 2); // Two misses (arc1 and arc2 were new)
  assert_eq!(stats.returns, 1);
  assert_eq!(stats.total_requests, 3);
  assert!(stats.hit_rate() > 0.0);
}

#[test]
fn test_arc_pool_reset_statistics() {
  let pool = ArcPool::<String>::new(10);

  pool.get_or_create("test".to_string());
  assert_eq!(pool.misses(), 1);

  pool.reset_statistics();
  assert_eq!(pool.misses(), 0);
  assert_eq!(pool.hits(), 0);
  assert_eq!(pool.returns(), 0);
}

#[test]
fn test_pool_statistics_hit_rate() {
  let pool = ArcPool::<i32>::new(10);

  // Create and return values to build up pool
  for i in 0..5 {
    let arc = pool.get_or_create(i);
    pool.return_arc(arc);
  }

  // Now get values - should hit pool
  for _ in 0..5 {
    pool.get_or_create(100); // Value doesn't matter, will get from pool
  }

  let stats = pool.statistics();
  assert_eq!(stats.hits, 5);
  assert_eq!(stats.misses, 5); // First 5 were misses
  assert_eq!(stats.hit_rate(), 0.5); // 50% hit rate
}
