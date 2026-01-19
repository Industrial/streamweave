//! Tests for comparison common utilities

use crate::nodes::comparison::common::compare_equal;
use std::any::Any;
use std::sync::Arc;

#[test]
fn test_compare_equal_i32() {
  let v1 = Arc::new(5i32) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(5i32) as Arc<dyn Any + Send + Sync>;
  let result = compare_equal(&v1, &v2).unwrap();
  assert!(*result.downcast::<bool>().unwrap());
}

#[test]
fn test_compare_equal_i32_not_equal() {
  let v1 = Arc::new(5i32) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(3i32) as Arc<dyn Any + Send + Sync>;
  let result = compare_equal(&v1, &v2).unwrap();
  assert!(!(*result.downcast::<bool>().unwrap()));
}

#[test]
fn test_compare_equal_f64() {
  let v1 = Arc::new(std::f64::consts::PI) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(std::f64::consts::PI) as Arc<dyn Any + Send + Sync>;
  let result = compare_equal(&v1, &v2).unwrap();
  assert!(*result.downcast::<bool>().unwrap());
}

#[test]
fn test_compare_equal_string() {
  let v1 = Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>;
  let result = compare_equal(&v1, &v2).unwrap();
  assert!(*result.downcast::<bool>().unwrap());
}

#[test]
fn test_compare_equal_type_promotion_i32_to_i64() {
  let v1 = Arc::new(5i32) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(5i64) as Arc<dyn Any + Send + Sync>;
  let result = compare_equal(&v1, &v2).unwrap();
  assert!(*result.downcast::<bool>().unwrap());
}

#[test]
fn test_compare_equal_type_promotion_i32_to_f64() {
  let v1 = Arc::new(5i32) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(5.0f64) as Arc<dyn Any + Send + Sync>;
  let result = compare_equal(&v1, &v2).unwrap();
  assert!(*result.downcast::<bool>().unwrap());
}

#[test]
fn test_compare_equal_incompatible_types() {
  let v1 = Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(5i32) as Arc<dyn Any + Send + Sync>;
  let result = compare_equal(&v1, &v2).unwrap();
  assert!(!(*result.downcast::<bool>().unwrap()));
}
