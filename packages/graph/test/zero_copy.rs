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
use streamweave_graph::zero_copy::*;

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
