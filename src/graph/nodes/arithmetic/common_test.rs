//! Tests for arithmetic common utilities

use crate::graph::nodes::arithmetic::common::add_values;
use std::any::Any;
use std::sync::Arc;

#[test]
fn test_add_values_i32() {
  let v1 = Arc::new(10i32) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(20i32) as Arc<dyn Any + Send + Sync>;
  let result = add_values(&v1, &v2).unwrap();
  assert_eq!(*result.downcast::<i32>().unwrap(), 30);
}

#[test]
fn test_add_values_i64() {
  let v1 = Arc::new(100i64) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(200i64) as Arc<dyn Any + Send + Sync>;
  let result = add_values(&v1, &v2).unwrap();
  assert_eq!(*result.downcast::<i64>().unwrap(), 300);
}

#[test]
fn test_add_values_f64() {
  let v1 = Arc::new(1.5f64) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(2.5f64) as Arc<dyn Any + Send + Sync>;
  let result = add_values(&v1, &v2).unwrap();
  let result_val = *result.downcast::<f64>().unwrap();
  assert!((result_val - 4.0).abs() < 0.001);
}

#[test]
fn test_add_values_type_promotion_i32_to_i64() {
  let v1 = Arc::new(10i32) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(20i64) as Arc<dyn Any + Send + Sync>;
  let result = add_values(&v1, &v2).unwrap();
  assert_eq!(*result.downcast::<i64>().unwrap(), 30);
}

#[test]
fn test_add_values_type_promotion_i32_to_f32() {
  let v1 = Arc::new(10i32) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(2.5f32) as Arc<dyn Any + Send + Sync>;
  let result = add_values(&v1, &v2).unwrap();
  let result_val = *result.downcast::<f32>().unwrap();
  assert!((result_val - 12.5).abs() < 0.001);
}

#[test]
fn test_add_values_overflow() {
  let v1 = Arc::new(i32::MAX) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new(1i32) as Arc<dyn Any + Send + Sync>;
  let result = add_values(&v1, &v2);
  assert!(result.is_err());
  assert!(result.unwrap_err().contains("overflow"));
}

#[test]
fn test_add_values_unsupported_type() {
  let v1 = Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>;
  let v2 = Arc::new("world".to_string()) as Arc<dyn Any + Send + Sync>;
  let result = add_values(&v1, &v2);
  assert!(result.is_err());
  assert!(result.unwrap_err().contains("Unsupported types"));
}
