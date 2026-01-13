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

/// Computes the minimum of two numeric values, handling type promotion.
///
/// This function attempts to downcast both values to numeric types and computes
/// their minimum. It handles:
/// - Integer types: i32, i64, u32, u64
/// - Floating point types: f32, f64
/// - Type promotion: smaller types are promoted to larger types when needed
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn min_values(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try same type comparisons first
  if let (Ok(arc_i32_1), Ok(arc_i32_2)) =
    (v1.clone().downcast::<i32>(), v2.clone().downcast::<i32>())
  {
    return Ok(Arc::new((*arc_i32_1).min(*arc_i32_2)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64_1), Ok(arc_i64_2)) =
    (v1.clone().downcast::<i64>(), v2.clone().downcast::<i64>())
  {
    return Ok(Arc::new((*arc_i64_1).min(*arc_i64_2)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_u32_1), Ok(arc_u32_2)) =
    (v1.clone().downcast::<u32>(), v2.clone().downcast::<u32>())
  {
    return Ok(Arc::new((*arc_u32_1).min(*arc_u32_2)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_u64_1), Ok(arc_u64_2)) =
    (v1.clone().downcast::<u64>(), v2.clone().downcast::<u64>())
  {
    return Ok(Arc::new((*arc_u64_1).min(*arc_u64_2)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32_1), Ok(arc_f32_2)) =
    (v1.clone().downcast::<f32>(), v2.clone().downcast::<f32>())
  {
    return Ok(Arc::new((*arc_f32_1).min(*arc_f32_2)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f64_1), Ok(arc_f64_2)) =
    (v1.clone().downcast::<f64>(), v2.clone().downcast::<f64>())
  {
    return Ok(Arc::new((*arc_f64_1).min(*arc_f64_2)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 min i64
  if let (Ok(arc_i32), Ok(arc_i64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<i64>()) {
    return Ok(Arc::new((*arc_i32 as i64).min(*arc_i64)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_i64), Ok(arc_i32)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new((*arc_i64).min(*arc_i32 as i64)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: u32 min u64
  if let (Ok(arc_u32), Ok(arc_u64)) = (v1.clone().downcast::<u32>(), v2.clone().downcast::<u64>()) {
    return Ok(Arc::new((*arc_u32 as u64).min(*arc_u64)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_u64), Ok(arc_u32)) = (v1.clone().downcast::<u64>(), v2.clone().downcast::<u32>()) {
    return Ok(Arc::new((*arc_u64).min(*arc_u32 as u64)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: integer min float
  if let (Ok(arc_i32), Ok(arc_f32)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new((*arc_i32 as f32).min(*arc_f32)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f32), Ok(arc_i32)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new((*arc_f32).min(*arc_i32 as f32)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 min f64
  if let (Ok(arc_i32), Ok(arc_f64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new((*arc_i32 as f64).min(*arc_f64)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_i32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new((*arc_f64).min(*arc_i32 as f64)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64), Ok(arc_f64)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new((*arc_i64 as f64).min(*arc_f64)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_i64)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i64>()) {
    return Ok(Arc::new((*arc_f64).min(*arc_i64 as f64)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32), Ok(arc_f64)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new((*arc_f32 as f64).min(*arc_f64)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_f32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new((*arc_f64).min(*arc_f32 as f64)) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported types for minimum: {} and {}",
    std::any::type_name_of_val(&**v1),
    std::any::type_name_of_val(&**v2)
  ))
}

/// Computes the maximum of two numeric values, handling type promotion.
///
/// This function attempts to downcast both values to numeric types and computes
/// their maximum. It handles:
/// - Integer types: i32, i64, u32, u64
/// - Floating point types: f32, f64
/// - Type promotion: smaller types are promoted to larger types when needed
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn max_values(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try same type comparisons first
  if let (Ok(arc_i32_1), Ok(arc_i32_2)) =
    (v1.clone().downcast::<i32>(), v2.clone().downcast::<i32>())
  {
    return Ok(Arc::new((*arc_i32_1).max(*arc_i32_2)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64_1), Ok(arc_i64_2)) =
    (v1.clone().downcast::<i64>(), v2.clone().downcast::<i64>())
  {
    return Ok(Arc::new((*arc_i64_1).max(*arc_i64_2)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_u32_1), Ok(arc_u32_2)) =
    (v1.clone().downcast::<u32>(), v2.clone().downcast::<u32>())
  {
    return Ok(Arc::new((*arc_u32_1).max(*arc_u32_2)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_u64_1), Ok(arc_u64_2)) =
    (v1.clone().downcast::<u64>(), v2.clone().downcast::<u64>())
  {
    return Ok(Arc::new((*arc_u64_1).max(*arc_u64_2)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32_1), Ok(arc_f32_2)) =
    (v1.clone().downcast::<f32>(), v2.clone().downcast::<f32>())
  {
    return Ok(Arc::new((*arc_f32_1).max(*arc_f32_2)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f64_1), Ok(arc_f64_2)) =
    (v1.clone().downcast::<f64>(), v2.clone().downcast::<f64>())
  {
    return Ok(Arc::new((*arc_f64_1).max(*arc_f64_2)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 max i64
  if let (Ok(arc_i32), Ok(arc_i64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<i64>()) {
    return Ok(Arc::new((*arc_i32 as i64).max(*arc_i64)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_i64), Ok(arc_i32)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new((*arc_i64).max(*arc_i32 as i64)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: u32 max u64
  if let (Ok(arc_u32), Ok(arc_u64)) = (v1.clone().downcast::<u32>(), v2.clone().downcast::<u64>()) {
    return Ok(Arc::new((*arc_u32 as u64).max(*arc_u64)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_u64), Ok(arc_u32)) = (v1.clone().downcast::<u64>(), v2.clone().downcast::<u32>()) {
    return Ok(Arc::new((*arc_u64).max(*arc_u32 as u64)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: integer max float
  if let (Ok(arc_i32), Ok(arc_f32)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new((*arc_i32 as f32).max(*arc_f32)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f32), Ok(arc_i32)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new((*arc_f32).max(*arc_i32 as f32)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 max f64
  if let (Ok(arc_i32), Ok(arc_f64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new((*arc_i32 as f64).max(*arc_f64)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_i32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new((*arc_f64).max(*arc_i32 as f64)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64), Ok(arc_f64)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new((*arc_i64 as f64).max(*arc_f64)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_i64)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i64>()) {
    return Ok(Arc::new((*arc_f64).max(*arc_i64 as f64)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32), Ok(arc_f64)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new((*arc_f32 as f64).max(*arc_f64)) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_f32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new((*arc_f64).max(*arc_f32 as f64)) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported types for maximum: {} and {}",
    std::any::type_name_of_val(&**v1),
    std::any::type_name_of_val(&**v2)
  ))
}

/// Rounds a numeric value to the nearest integer.
///
/// This function attempts to downcast the value to a numeric type and rounds
/// it to the nearest integer. It handles:
/// - Integer types: i32, i64, u32, u64 (returns as-is)
/// - Floating point types: f32, f64 (rounds to nearest integer)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn round_value(
  value: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Integers are already "rounded", just return them
  if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    return Ok(Arc::new(*arc_i32) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    return Ok(Arc::new(*arc_i64) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    return Ok(Arc::new(*arc_u32) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    return Ok(Arc::new(*arc_u64) as Arc<dyn Any + Send + Sync>);
  }

  // Round floating point values
  if let Ok(arc_f32) = value.clone().downcast::<f32>() {
    return Ok(Arc::new(arc_f32.round() as i64) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_f64) = value.clone().downcast::<f64>() {
    return Ok(Arc::new(arc_f64.round() as i64) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported type for rounding: {}",
    std::any::type_name_of_val(&**value)
  ))
}

/// Floors a numeric value to the largest integer less than or equal to it.
///
/// This function attempts to downcast the value to a numeric type and floors
/// it. It handles:
/// - Integer types: i32, i64, u32, u64 (returns as-is)
/// - Floating point types: f32, f64 (floors to largest integer <= value)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn floor_value(
  value: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Integers are already "floored", just return them
  if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    return Ok(Arc::new(*arc_i32) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    return Ok(Arc::new(*arc_i64) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    return Ok(Arc::new(*arc_u32) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    return Ok(Arc::new(*arc_u64) as Arc<dyn Any + Send + Sync>);
  }

  // Floor floating point values
  if let Ok(arc_f32) = value.clone().downcast::<f32>() {
    return Ok(Arc::new(arc_f32.floor() as i64) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_f64) = value.clone().downcast::<f64>() {
    return Ok(Arc::new(arc_f64.floor() as i64) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported type for floor: {}",
    std::any::type_name_of_val(&**value)
  ))
}

/// Ceils a numeric value to the smallest integer greater than or equal to it.
///
/// This function attempts to downcast the value to a numeric type and ceils
/// it. It handles:
/// - Integer types: i32, i64, u32, u64 (returns as-is)
/// - Floating point types: f32, f64 (ceils to smallest integer >= value)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn ceil_value(
  value: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Integers are already "ceiled", just return them
  if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    return Ok(Arc::new(*arc_i32) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    return Ok(Arc::new(*arc_i64) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_u32) = value.clone().downcast::<u32>() {
    return Ok(Arc::new(*arc_u32) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_u64) = value.clone().downcast::<u64>() {
    return Ok(Arc::new(*arc_u64) as Arc<dyn Any + Send + Sync>);
  }

  // Ceil floating point values
  if let Ok(arc_f32) = value.clone().downcast::<f32>() {
    return Ok(Arc::new(arc_f32.ceil() as i64) as Arc<dyn Any + Send + Sync>);
  }
  if let Ok(arc_f64) = value.clone().downcast::<f64>() {
    return Ok(Arc::new(arc_f64.ceil() as i64) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported type for ceil: {}",
    std::any::type_name_of_val(&**value)
  ))
}
