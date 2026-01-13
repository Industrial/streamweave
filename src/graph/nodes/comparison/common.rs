//! # Comparison Common Utilities
//!
//! Shared utilities for comparison operation nodes.

use std::any::Any;
use std::sync::Arc;

/// Compares two values for equality, handling type promotion.
///
/// This function attempts to downcast both values to the same type and performs
/// equality comparison. It handles:
/// - Integer types: i32, i64, u32, u64
/// - Floating point types: f32, f64
/// - String types
/// - Boolean types
/// - Type promotion: smaller types are promoted to larger types when needed
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (boolean) or an error string.
pub fn compare_equal(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try same type comparisons first
  if let (Ok(arc_i32_1), Ok(arc_i32_2)) =
    (v1.clone().downcast::<i32>(), v2.clone().downcast::<i32>())
  {
    return Ok(Arc::new(*arc_i32_1 == *arc_i32_2) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64_1), Ok(arc_i64_2)) =
    (v1.clone().downcast::<i64>(), v2.clone().downcast::<i64>())
  {
    return Ok(Arc::new(*arc_i64_1 == *arc_i64_2) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_u32_1), Ok(arc_u32_2)) =
    (v1.clone().downcast::<u32>(), v2.clone().downcast::<u32>())
  {
    return Ok(Arc::new(*arc_u32_1 == *arc_u32_2) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_u64_1), Ok(arc_u64_2)) =
    (v1.clone().downcast::<u64>(), v2.clone().downcast::<u64>())
  {
    return Ok(Arc::new(*arc_u64_1 == *arc_u64_2) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32_1), Ok(arc_f32_2)) =
    (v1.clone().downcast::<f32>(), v2.clone().downcast::<f32>())
  {
    return Ok(
      Arc::new((*arc_f32_1 - *arc_f32_2).abs() < f32::EPSILON) as Arc<dyn Any + Send + Sync>
    );
  }

  if let (Ok(arc_f64_1), Ok(arc_f64_2)) =
    (v1.clone().downcast::<f64>(), v2.clone().downcast::<f64>())
  {
    return Ok(
      Arc::new((*arc_f64_1 - *arc_f64_2).abs() < f64::EPSILON) as Arc<dyn Any + Send + Sync>
    );
  }

  if let (Ok(arc_str_1), Ok(arc_str_2)) = (
    v1.clone().downcast::<String>(),
    v2.clone().downcast::<String>(),
  ) {
    return Ok(Arc::new(*arc_str_1 == *arc_str_2) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_bool_1), Ok(arc_bool_2)) =
    (v1.clone().downcast::<bool>(), v2.clone().downcast::<bool>())
  {
    return Ok(Arc::new(*arc_bool_1 == *arc_bool_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 == i64
  if let (Ok(arc_i32), Ok(arc_i64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<i64>()) {
    return Ok(Arc::new((*arc_i32 as i64) == *arc_i64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_i64), Ok(arc_i32)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new(*arc_i64 == (*arc_i32 as i64)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: u32 == u64
  if let (Ok(arc_u32), Ok(arc_u64)) = (v1.clone().downcast::<u32>(), v2.clone().downcast::<u64>()) {
    return Ok(Arc::new((*arc_u32 as u64) == *arc_u64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_u64), Ok(arc_u32)) = (v1.clone().downcast::<u64>(), v2.clone().downcast::<u32>()) {
    return Ok(Arc::new(*arc_u64 == (*arc_u32 as u64)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: integer == float (with epsilon comparison for floats)
  if let (Ok(arc_i32), Ok(arc_f32)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f32>()) {
    return Ok(
      Arc::new((*arc_i32 as f32 - *arc_f32).abs() < f32::EPSILON) as Arc<dyn Any + Send + Sync>
    );
  }
  if let (Ok(arc_f32), Ok(arc_i32)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<i32>()) {
    return Ok(
      Arc::new((*arc_f32 - *arc_i32 as f32).abs() < f32::EPSILON) as Arc<dyn Any + Send + Sync>
    );
  }

  if let (Ok(arc_i32), Ok(arc_f64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f64>()) {
    return Ok(
      Arc::new((*arc_i32 as f64 - *arc_f64).abs() < f64::EPSILON) as Arc<dyn Any + Send + Sync>
    );
  }
  if let (Ok(arc_f64), Ok(arc_i32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i32>()) {
    return Ok(
      Arc::new((*arc_f64 - *arc_i32 as f64).abs() < f64::EPSILON) as Arc<dyn Any + Send + Sync>
    );
  }

  if let (Ok(arc_i64), Ok(arc_f64)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<f64>()) {
    return Ok(
      Arc::new((*arc_i64 as f64 - *arc_f64).abs() < f64::EPSILON) as Arc<dyn Any + Send + Sync>
    );
  }
  if let (Ok(arc_f64), Ok(arc_i64)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i64>()) {
    return Ok(
      Arc::new((*arc_f64 - *arc_i64 as f64).abs() < f64::EPSILON) as Arc<dyn Any + Send + Sync>
    );
  }

  if let (Ok(arc_f32), Ok(arc_f64)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<f64>()) {
    return Ok(
      Arc::new((*arc_f32 as f64 - *arc_f64).abs() < f64::EPSILON) as Arc<dyn Any + Send + Sync>
    );
  }
  if let (Ok(arc_f64), Ok(arc_f32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<f32>()) {
    return Ok(
      Arc::new((*arc_f64 - *arc_f32 as f64).abs() < f64::EPSILON) as Arc<dyn Any + Send + Sync>
    );
  }

  // If types are completely incompatible, return false
  // This allows comparison nodes to work with any types, returning false for incompatible types
  Ok(Arc::new(false) as Arc<dyn Any + Send + Sync>)
}

/// Compares two values for inequality, handling type promotion.
///
/// This function attempts to downcast both values to the same type and performs
/// inequality comparison. It handles:
/// - Integer types: i32, i64, u32, u64
/// - Floating point types: f32, f64
/// - String types
/// - Boolean types
/// - Type promotion: smaller types are promoted to larger types when needed
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (boolean) or an error string.
pub fn compare_not_equal(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Use compare_equal and negate the result
  match compare_equal(v1, v2) {
    Ok(result) => {
      if let Ok(arc_bool) = result.downcast::<bool>() {
        return Ok(Arc::new(!*arc_bool) as Arc<dyn Any + Send + Sync>);
      }
      // Should not happen, but handle gracefully
      Ok(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    }
    Err(e) => Err(e),
  }
}

/// Compares two values for greater than, handling type promotion.
///
/// This function attempts to downcast both values to numeric types and performs
/// greater than comparison. It handles:
/// - Integer types: i32, i64, u32, u64
/// - Floating point types: f32, f64
/// - Type promotion: smaller types are promoted to larger types when needed
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (boolean) or an error string.
/// For incompatible types, returns false (no error).
pub fn compare_greater_than(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try same type comparisons first
  if let (Ok(arc_i32_1), Ok(arc_i32_2)) =
    (v1.clone().downcast::<i32>(), v2.clone().downcast::<i32>())
  {
    return Ok(Arc::new(*arc_i32_1 > *arc_i32_2) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64_1), Ok(arc_i64_2)) =
    (v1.clone().downcast::<i64>(), v2.clone().downcast::<i64>())
  {
    return Ok(Arc::new(*arc_i64_1 > *arc_i64_2) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_u32_1), Ok(arc_u32_2)) =
    (v1.clone().downcast::<u32>(), v2.clone().downcast::<u32>())
  {
    return Ok(Arc::new(*arc_u32_1 > *arc_u32_2) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_u64_1), Ok(arc_u64_2)) =
    (v1.clone().downcast::<u64>(), v2.clone().downcast::<u64>())
  {
    return Ok(Arc::new(*arc_u64_1 > *arc_u64_2) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32_1), Ok(arc_f32_2)) =
    (v1.clone().downcast::<f32>(), v2.clone().downcast::<f32>())
  {
    return Ok(Arc::new(*arc_f32_1 > *arc_f32_2) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f64_1), Ok(arc_f64_2)) =
    (v1.clone().downcast::<f64>(), v2.clone().downcast::<f64>())
  {
    return Ok(Arc::new(*arc_f64_1 > *arc_f64_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 > i64
  if let (Ok(arc_i32), Ok(arc_i64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<i64>()) {
    return Ok(Arc::new((*arc_i32 as i64) > *arc_i64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_i64), Ok(arc_i32)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new(*arc_i64 > (*arc_i32 as i64)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: u32 > u64
  if let (Ok(arc_u32), Ok(arc_u64)) = (v1.clone().downcast::<u32>(), v2.clone().downcast::<u64>()) {
    return Ok(Arc::new((*arc_u32 as u64) > *arc_u64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_u64), Ok(arc_u32)) = (v1.clone().downcast::<u64>(), v2.clone().downcast::<u32>()) {
    return Ok(Arc::new(*arc_u64 > (*arc_u32 as u64)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: integer > float
  if let (Ok(arc_i32), Ok(arc_f32)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new((*arc_i32 as f32) > *arc_f32) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f32), Ok(arc_i32)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new(*arc_f32 > (*arc_i32 as f32)) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 > f64
  if let (Ok(arc_i32), Ok(arc_f64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new((*arc_i32 as f64) > *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_i32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new(*arc_f64 > (*arc_i32 as f64)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64), Ok(arc_f64)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new((*arc_i64 as f64) > *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_i64)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i64>()) {
    return Ok(Arc::new(*arc_f64 > (*arc_i64 as f64)) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32), Ok(arc_f64)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new((*arc_f32 as f64) > *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_f32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new(*arc_f64 > (*arc_f32 as f64)) as Arc<dyn Any + Send + Sync>);
  }

  // If types are completely incompatible, return false
  // This allows comparison nodes to work with any types, returning false for incompatible types
  Ok(Arc::new(false) as Arc<dyn Any + Send + Sync>)
}

/// Compares two values for greater than or equal, handling type promotion.
///
/// This function attempts to downcast both values to numeric types and performs
/// greater than or equal comparison. It handles:
/// - Integer types: i32, i64, u32, u64
/// - Floating point types: f32, f64
/// - Type promotion: smaller types are promoted to larger types when needed
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (boolean) or an error string.
/// For incompatible types, returns false (no error).
pub fn compare_greater_than_or_equal(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Use compare_greater_than and compare_equal, then combine with OR
  let gt_result = compare_greater_than(v1, v2)?;
  let eq_result = compare_equal(v1, v2)?;

  if let (Ok(arc_gt), Ok(arc_eq)) = (gt_result.downcast::<bool>(), eq_result.downcast::<bool>()) {
    return Ok(Arc::new(*arc_gt || *arc_eq) as Arc<dyn Any + Send + Sync>);
  }

  // Should not happen, but handle gracefully
  Ok(Arc::new(false) as Arc<dyn Any + Send + Sync>)
}
