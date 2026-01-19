//! Tests for math common utilities

use crate::nodes::math::common::abs_value;
use std::any::Any;
use std::sync::Arc;

#[test]
fn test_abs_value_i32_positive() {
  let v = Arc::new(5i32) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v).unwrap();
  assert_eq!(*result.downcast::<i32>().unwrap(), 5);
}

#[test]
fn test_abs_value_i32_negative() {
  let v = Arc::new(-5i32) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v).unwrap();
  assert_eq!(*result.downcast::<i32>().unwrap(), 5);
}

#[test]
fn test_abs_value_i32_zero() {
  let v = Arc::new(0i32) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v).unwrap();
  assert_eq!(*result.downcast::<i32>().unwrap(), 0);
}

#[test]
fn test_abs_value_i64_positive() {
  let v = Arc::new(100i64) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v).unwrap();
  assert_eq!(*result.downcast::<i64>().unwrap(), 100);
}

#[test]
fn test_abs_value_i64_negative() {
  let v = Arc::new(-100i64) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v).unwrap();
  assert_eq!(*result.downcast::<i64>().unwrap(), 100);
}

#[test]
fn test_abs_value_u32() {
  let v = Arc::new(5u32) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v).unwrap();
  assert_eq!(*result.downcast::<u32>().unwrap(), 5);
}

#[test]
fn test_abs_value_u64() {
  let v = Arc::new(100u64) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v).unwrap();
  assert_eq!(*result.downcast::<u64>().unwrap(), 100);
}

#[test]
fn test_abs_value_f32_positive() {
  let v = Arc::new(std::f32::consts::PI) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v).unwrap();
  assert_eq!(*result.downcast::<f32>().unwrap(), std::f32::consts::PI);
}

#[test]
fn test_abs_value_f32_negative() {
  let v = Arc::new(-std::f32::consts::PI) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v).unwrap();
  assert_eq!(*result.downcast::<f32>().unwrap(), std::f32::consts::PI);
}

#[test]
fn test_abs_value_f64_positive() {
  let v = Arc::new(std::f64::consts::PI) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v).unwrap();
  assert_eq!(*result.downcast::<f64>().unwrap(), std::f64::consts::PI);
}

#[test]
fn test_abs_value_f64_negative() {
  let v = Arc::new(-std::f64::consts::PI) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v).unwrap();
  assert_eq!(*result.downcast::<f64>().unwrap(), std::f64::consts::PI);
}

#[test]
fn test_abs_value_unsupported_type() {
  let v = Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>;
  let result = abs_value(&v);
  assert!(result.is_err());
  assert!(result.unwrap_err().contains("Unsupported type"));
}
