//! # Math Common Utilities
//!
//! Shared utilities for math function nodes.

use std::any::Any;
use std::sync::Arc;

/// Computes the absolute value of a numeric value.
///
/// This function attempts to downcast the value to a numeric type and computes
/// its absolute value. It handles:
/// - Integer types: i32, i64 (absolute value of signed integers)
/// - Unsigned types: u32, u64 (already non-negative, returns as-is)
/// - Floating point types: f32, f64
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn abs_value(v: &Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try i32
  if let Ok(arc_i32) = v.clone().downcast::<i32>() {
    return Ok(Arc::new(arc_i32.abs()) as Arc<dyn Any + Send + Sync>);
  }

  // Try i64
  if let Ok(arc_i64) = v.clone().downcast::<i64>() {
    return Ok(Arc::new(arc_i64.abs()) as Arc<dyn Any + Send + Sync>);
  }

  // Try u32 (already non-negative)
  if let Ok(arc_u32) = v.clone().downcast::<u32>() {
    return Ok(Arc::new(*arc_u32) as Arc<dyn Any + Send + Sync>);
  }

  // Try u64 (already non-negative)
  if let Ok(arc_u64) = v.clone().downcast::<u64>() {
    return Ok(Arc::new(*arc_u64) as Arc<dyn Any + Send + Sync>);
  }

  // Try f32
  if let Ok(arc_f32) = v.clone().downcast::<f32>() {
    return Ok(Arc::new(arc_f32.abs()) as Arc<dyn Any + Send + Sync>);
  }

  // Try f64
  if let Ok(arc_f64) = v.clone().downcast::<f64>() {
    return Ok(Arc::new(arc_f64.abs()) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported type for absolute value: {}",
    std::any::type_name_of_val(&**v)
  ))
}
