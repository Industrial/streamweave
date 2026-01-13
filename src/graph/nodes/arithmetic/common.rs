//! # Arithmetic Common Utilities
//!
//! Shared utilities for arithmetic operation nodes.

use std::any::Any;
use std::sync::Arc;

/// Performs addition on two numeric values, handling type promotion and overflow.
///
/// This function attempts to downcast both values to numeric types and performs
/// addition with appropriate type promotion. It handles:
/// - Integer types: i32, i64, u32, u64 (with overflow checking)
/// - Floating point types: f32, f64
/// - Type promotion: smaller types are promoted to larger types when needed
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn add_values(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try i32 + i32
  if let (Ok(arc_i32_1), Ok(arc_i32_2)) =
    (v1.clone().downcast::<i32>(), v2.clone().downcast::<i32>())
  {
    match arc_i32_1.checked_add(*arc_i32_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i32 addition".to_string()),
    }
  }

  // Try i64 + i64
  if let (Ok(arc_i64_1), Ok(arc_i64_2)) =
    (v1.clone().downcast::<i64>(), v2.clone().downcast::<i64>())
  {
    match arc_i64_1.checked_add(*arc_i64_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i64 addition".to_string()),
    }
  }

  // Try u32 + u32
  if let (Ok(arc_u32_1), Ok(arc_u32_2)) =
    (v1.clone().downcast::<u32>(), v2.clone().downcast::<u32>())
  {
    match arc_u32_1.checked_add(*arc_u32_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u32 addition".to_string()),
    }
  }

  // Try u64 + u64
  if let (Ok(arc_u64_1), Ok(arc_u64_2)) =
    (v1.clone().downcast::<u64>(), v2.clone().downcast::<u64>())
  {
    match arc_u64_1.checked_add(*arc_u64_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u64 addition".to_string()),
    }
  }

  // Try f32 + f32
  if let (Ok(arc_f32_1), Ok(arc_f32_2)) =
    (v1.clone().downcast::<f32>(), v2.clone().downcast::<f32>())
  {
    return Ok(Arc::new(*arc_f32_1 + *arc_f32_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try f64 + f64
  if let (Ok(arc_f64_1), Ok(arc_f64_2)) =
    (v1.clone().downcast::<f64>(), v2.clone().downcast::<f64>())
  {
    return Ok(Arc::new(*arc_f64_1 + *arc_f64_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 + i64 -> i64
  if let (Ok(arc_i32), Ok(arc_i64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<i64>()) {
    match (*arc_i32 as i64).checked_add(*arc_i64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i32 + i64 addition".to_string()),
    }
  }
  if let (Ok(arc_i64), Ok(arc_i32)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<i32>()) {
    match arc_i64.checked_add(*arc_i32 as i64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i64 + i32 addition".to_string()),
    }
  }

  // Try type promotion: u32 + u64 -> u64
  if let (Ok(arc_u32), Ok(arc_u64)) = (v1.clone().downcast::<u32>(), v2.clone().downcast::<u64>()) {
    match (*arc_u32 as u64).checked_add(*arc_u64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u32 + u64 addition".to_string()),
    }
  }
  if let (Ok(arc_u64), Ok(arc_u32)) = (v1.clone().downcast::<u64>(), v2.clone().downcast::<u32>()) {
    match arc_u64.checked_add(*arc_u32 as u64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u64 + u32 addition".to_string()),
    }
  }

  // Try type promotion: integer + float -> float
  if let (Ok(arc_i32), Ok(arc_f32)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new(*arc_i32 as f32 + *arc_f32) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f32), Ok(arc_i32)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new(*arc_f32 + *arc_i32 as f32) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64), Ok(arc_f64)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new(*arc_i64 as f64 + *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_i64)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i64>()) {
    return Ok(Arc::new(*arc_f64 + *arc_i64 as f64) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32), Ok(arc_f64)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new(*arc_f32 as f64 + *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_f32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new(*arc_f64 + *arc_f32 as f64) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported types for addition: {} + {}",
    std::any::type_name_of_val(&**v1),
    std::any::type_name_of_val(&**v2)
  ))
}

/// Performs subtraction on two numeric values, handling type promotion and overflow.
///
/// This function attempts to downcast both values to numeric types and performs
/// subtraction with appropriate type promotion. It handles:
/// - Integer types: i32, i64, u32, u64 (with overflow checking)
/// - Floating point types: f32, f64
/// - Type promotion: smaller types are promoted to larger types when needed
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn subtract_values(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try i32 - i32
  if let (Ok(arc_i32_1), Ok(arc_i32_2)) =
    (v1.clone().downcast::<i32>(), v2.clone().downcast::<i32>())
  {
    match arc_i32_1.checked_sub(*arc_i32_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow/underflow in i32 subtraction".to_string()),
    }
  }

  // Try i64 - i64
  if let (Ok(arc_i64_1), Ok(arc_i64_2)) =
    (v1.clone().downcast::<i64>(), v2.clone().downcast::<i64>())
  {
    match arc_i64_1.checked_sub(*arc_i64_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow/underflow in i64 subtraction".to_string()),
    }
  }

  // Try u32 - u32
  if let (Ok(arc_u32_1), Ok(arc_u32_2)) =
    (v1.clone().downcast::<u32>(), v2.clone().downcast::<u32>())
  {
    match arc_u32_1.checked_sub(*arc_u32_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer underflow in u32 subtraction".to_string()),
    }
  }

  // Try u64 - u64
  if let (Ok(arc_u64_1), Ok(arc_u64_2)) =
    (v1.clone().downcast::<u64>(), v2.clone().downcast::<u64>())
  {
    match arc_u64_1.checked_sub(*arc_u64_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer underflow in u64 subtraction".to_string()),
    }
  }

  // Try f32 - f32
  if let (Ok(arc_f32_1), Ok(arc_f32_2)) =
    (v1.clone().downcast::<f32>(), v2.clone().downcast::<f32>())
  {
    return Ok(Arc::new(*arc_f32_1 - *arc_f32_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try f64 - f64
  if let (Ok(arc_f64_1), Ok(arc_f64_2)) =
    (v1.clone().downcast::<f64>(), v2.clone().downcast::<f64>())
  {
    return Ok(Arc::new(*arc_f64_1 - *arc_f64_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 - i64 -> i64
  if let (Ok(arc_i32), Ok(arc_i64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<i64>()) {
    match (*arc_i32 as i64).checked_sub(*arc_i64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow/underflow in i32 - i64 subtraction".to_string()),
    }
  }
  if let (Ok(arc_i64), Ok(arc_i32)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<i32>()) {
    match arc_i64.checked_sub(*arc_i32 as i64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow/underflow in i64 - i32 subtraction".to_string()),
    }
  }

  // Try type promotion: u32 - u64 -> u64
  if let (Ok(arc_u32), Ok(arc_u64)) = (v1.clone().downcast::<u32>(), v2.clone().downcast::<u64>()) {
    match (*arc_u32 as u64).checked_sub(*arc_u64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer underflow in u32 - u64 subtraction".to_string()),
    }
  }
  if let (Ok(arc_u64), Ok(arc_u32)) = (v1.clone().downcast::<u64>(), v2.clone().downcast::<u32>()) {
    match arc_u64.checked_sub(*arc_u32 as u64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer underflow in u64 - u32 subtraction".to_string()),
    }
  }

  // Try type promotion: integer - float -> float
  if let (Ok(arc_i32), Ok(arc_f32)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new(*arc_i32 as f32 - *arc_f32) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f32), Ok(arc_i32)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new(*arc_f32 - *arc_i32 as f32) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64), Ok(arc_f64)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new(*arc_i64 as f64 - *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_i64)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i64>()) {
    return Ok(Arc::new(*arc_f64 - *arc_i64 as f64) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32), Ok(arc_f64)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new(*arc_f32 as f64 - *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_f32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new(*arc_f64 - *arc_f32 as f64) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported types for subtraction: {} - {}",
    std::any::type_name_of_val(&**v1),
    std::any::type_name_of_val(&**v2)
  ))
}

/// Performs multiplication on two numeric values, handling type promotion and overflow.
///
/// This function attempts to downcast both values to numeric types and performs
/// multiplication with appropriate type promotion. It handles:
/// - Integer types: i32, i64, u32, u64 (with overflow checking)
/// - Floating point types: f32, f64
/// - Type promotion: smaller types are promoted to larger types when needed
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn multiply_values(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try i32 * i32
  if let (Ok(arc_i32_1), Ok(arc_i32_2)) =
    (v1.clone().downcast::<i32>(), v2.clone().downcast::<i32>())
  {
    match arc_i32_1.checked_mul(*arc_i32_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i32 multiplication".to_string()),
    }
  }

  // Try i64 * i64
  if let (Ok(arc_i64_1), Ok(arc_i64_2)) =
    (v1.clone().downcast::<i64>(), v2.clone().downcast::<i64>())
  {
    match arc_i64_1.checked_mul(*arc_i64_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i64 multiplication".to_string()),
    }
  }

  // Try u32 * u32
  if let (Ok(arc_u32_1), Ok(arc_u32_2)) =
    (v1.clone().downcast::<u32>(), v2.clone().downcast::<u32>())
  {
    match arc_u32_1.checked_mul(*arc_u32_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u32 multiplication".to_string()),
    }
  }

  // Try u64 * u64
  if let (Ok(arc_u64_1), Ok(arc_u64_2)) =
    (v1.clone().downcast::<u64>(), v2.clone().downcast::<u64>())
  {
    match arc_u64_1.checked_mul(*arc_u64_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u64 multiplication".to_string()),
    }
  }

  // Try f32 * f32
  if let (Ok(arc_f32_1), Ok(arc_f32_2)) =
    (v1.clone().downcast::<f32>(), v2.clone().downcast::<f32>())
  {
    return Ok(Arc::new(*arc_f32_1 * *arc_f32_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try f64 * f64
  if let (Ok(arc_f64_1), Ok(arc_f64_2)) =
    (v1.clone().downcast::<f64>(), v2.clone().downcast::<f64>())
  {
    return Ok(Arc::new(*arc_f64_1 * *arc_f64_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 * i64 -> i64
  if let (Ok(arc_i32), Ok(arc_i64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<i64>()) {
    match (*arc_i32 as i64).checked_mul(*arc_i64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i32 * i64 multiplication".to_string()),
    }
  }
  if let (Ok(arc_i64), Ok(arc_i32)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<i32>()) {
    match arc_i64.checked_mul(*arc_i32 as i64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i64 * i32 multiplication".to_string()),
    }
  }

  // Try type promotion: u32 * u64 -> u64
  if let (Ok(arc_u32), Ok(arc_u64)) = (v1.clone().downcast::<u32>(), v2.clone().downcast::<u64>()) {
    match (*arc_u32 as u64).checked_mul(*arc_u64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u32 * u64 multiplication".to_string()),
    }
  }
  if let (Ok(arc_u64), Ok(arc_u32)) = (v1.clone().downcast::<u64>(), v2.clone().downcast::<u32>()) {
    match arc_u64.checked_mul(*arc_u32 as u64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u64 * u32 multiplication".to_string()),
    }
  }

  // Try type promotion: integer * float -> float
  if let (Ok(arc_i32), Ok(arc_f32)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new(*arc_i32 as f32 * *arc_f32) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f32), Ok(arc_i32)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<i32>()) {
    return Ok(Arc::new(*arc_f32 * *arc_i32 as f32) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64), Ok(arc_f64)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new(*arc_i64 as f64 * *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_i64)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i64>()) {
    return Ok(Arc::new(*arc_f64 * *arc_i64 as f64) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32), Ok(arc_f64)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<f64>()) {
    return Ok(Arc::new(*arc_f32 as f64 * *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_f32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<f32>()) {
    return Ok(Arc::new(*arc_f64 * *arc_f32 as f64) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported types for multiplication: {} * {}",
    std::any::type_name_of_val(&**v1),
    std::any::type_name_of_val(&**v2)
  ))
}

/// Performs division on two numeric values, handling type promotion and division by zero.
///
/// This function attempts to downcast both values to numeric types and performs
/// division with appropriate type promotion. It handles:
/// - Integer types: i32, i64, u32, u64 (with division by zero checking)
/// - Floating point types: f32, f64
/// - Type promotion: smaller types are promoted to larger types when needed
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn divide_values(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try i32 / i32
  if let (Ok(arc_i32_1), Ok(arc_i32_2)) =
    (v1.clone().downcast::<i32>(), v2.clone().downcast::<i32>())
  {
    if *arc_i32_2 == 0 {
      return Err("Division by zero in i32 division".to_string());
    }
    match arc_i32_1.checked_div(*arc_i32_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i32 division".to_string()),
    }
  }

  // Try i64 / i64
  if let (Ok(arc_i64_1), Ok(arc_i64_2)) =
    (v1.clone().downcast::<i64>(), v2.clone().downcast::<i64>())
  {
    if *arc_i64_2 == 0 {
      return Err("Division by zero in i64 division".to_string());
    }
    match arc_i64_1.checked_div(*arc_i64_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i64 division".to_string()),
    }
  }

  // Try u32 / u32
  if let (Ok(arc_u32_1), Ok(arc_u32_2)) =
    (v1.clone().downcast::<u32>(), v2.clone().downcast::<u32>())
  {
    if *arc_u32_2 == 0 {
      return Err("Division by zero in u32 division".to_string());
    }
    match arc_u32_1.checked_div(*arc_u32_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u32 division".to_string()),
    }
  }

  // Try u64 / u64
  if let (Ok(arc_u64_1), Ok(arc_u64_2)) =
    (v1.clone().downcast::<u64>(), v2.clone().downcast::<u64>())
  {
    if *arc_u64_2 == 0 {
      return Err("Division by zero in u64 division".to_string());
    }
    match arc_u64_1.checked_div(*arc_u64_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u64 division".to_string()),
    }
  }

  // Try f32 / f32
  if let (Ok(arc_f32_1), Ok(arc_f32_2)) =
    (v1.clone().downcast::<f32>(), v2.clone().downcast::<f32>())
  {
    if *arc_f32_2 == 0.0 {
      return Err("Division by zero in f32 division".to_string());
    }
    return Ok(Arc::new(*arc_f32_1 / *arc_f32_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try f64 / f64
  if let (Ok(arc_f64_1), Ok(arc_f64_2)) =
    (v1.clone().downcast::<f64>(), v2.clone().downcast::<f64>())
  {
    if *arc_f64_2 == 0.0 {
      return Err("Division by zero in f64 division".to_string());
    }
    return Ok(Arc::new(*arc_f64_1 / *arc_f64_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 / i64 -> i64
  if let (Ok(arc_i32), Ok(arc_i64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<i64>()) {
    if *arc_i64 == 0 {
      return Err("Division by zero in i32 / i64 division".to_string());
    }
    match (*arc_i32 as i64).checked_div(*arc_i64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i32 / i64 division".to_string()),
    }
  }
  if let (Ok(arc_i64), Ok(arc_i32)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<i32>()) {
    if *arc_i32 == 0 {
      return Err("Division by zero in i64 / i32 division".to_string());
    }
    match arc_i64.checked_div(*arc_i32 as i64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i64 / i32 division".to_string()),
    }
  }

  // Try type promotion: u32 / u64 -> u64
  if let (Ok(arc_u32), Ok(arc_u64)) = (v1.clone().downcast::<u32>(), v2.clone().downcast::<u64>()) {
    if *arc_u64 == 0 {
      return Err("Division by zero in u32 / u64 division".to_string());
    }
    match (*arc_u32 as u64).checked_div(*arc_u64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u32 / u64 division".to_string()),
    }
  }
  if let (Ok(arc_u64), Ok(arc_u32)) = (v1.clone().downcast::<u64>(), v2.clone().downcast::<u32>()) {
    if *arc_u32 == 0 {
      return Err("Division by zero in u64 / u32 division".to_string());
    }
    match arc_u64.checked_div(*arc_u32 as u64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u64 / u32 division".to_string()),
    }
  }

  // Try type promotion: integer / float -> float
  if let (Ok(arc_i32), Ok(arc_f32)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f32>()) {
    if *arc_f32 == 0.0 {
      return Err("Division by zero in i32 / f32 division".to_string());
    }
    return Ok(Arc::new(*arc_i32 as f32 / *arc_f32) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f32), Ok(arc_i32)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<i32>()) {
    if *arc_i32 == 0 {
      return Err("Division by zero in f32 / i32 division".to_string());
    }
    return Ok(Arc::new(*arc_f32 / *arc_i32 as f32) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64), Ok(arc_f64)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<f64>()) {
    if *arc_f64 == 0.0 {
      return Err("Division by zero in i64 / f64 division".to_string());
    }
    return Ok(Arc::new(*arc_i64 as f64 / *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_i64)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i64>()) {
    if *arc_i64 == 0 {
      return Err("Division by zero in f64 / i64 division".to_string());
    }
    return Ok(Arc::new(*arc_f64 / *arc_i64 as f64) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32), Ok(arc_f64)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<f64>()) {
    if *arc_f64 == 0.0 {
      return Err("Division by zero in f32 / f64 division".to_string());
    }
    return Ok(Arc::new(*arc_f32 as f64 / *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_f32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<f32>()) {
    if *arc_f32 == 0.0 {
      return Err("Division by zero in f64 / f32 division".to_string());
    }
    return Ok(Arc::new(*arc_f64 / *arc_f32 as f64) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported types for division: {} / {}",
    std::any::type_name_of_val(&**v1),
    std::any::type_name_of_val(&**v2)
  ))
}

/// Performs modulo operation on two numeric values, handling type promotion and division by zero.
///
/// This function attempts to downcast both values to numeric types and performs
/// modulo operation with appropriate type promotion. It handles:
/// - Integer types: i32, i64, u32, u64 (with division by zero checking)
/// - Floating point types: f32, f64
/// - Type promotion: smaller types are promoted to larger types when needed
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn modulo_values(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try i32 % i32
  if let (Ok(arc_i32_1), Ok(arc_i32_2)) =
    (v1.clone().downcast::<i32>(), v2.clone().downcast::<i32>())
  {
    if *arc_i32_2 == 0 {
      return Err("Division by zero in i32 modulo".to_string());
    }
    match arc_i32_1.checked_rem(*arc_i32_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i32 modulo".to_string()),
    }
  }

  // Try i64 % i64
  if let (Ok(arc_i64_1), Ok(arc_i64_2)) =
    (v1.clone().downcast::<i64>(), v2.clone().downcast::<i64>())
  {
    if *arc_i64_2 == 0 {
      return Err("Division by zero in i64 modulo".to_string());
    }
    match arc_i64_1.checked_rem(*arc_i64_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i64 modulo".to_string()),
    }
  }

  // Try u32 % u32
  if let (Ok(arc_u32_1), Ok(arc_u32_2)) =
    (v1.clone().downcast::<u32>(), v2.clone().downcast::<u32>())
  {
    if *arc_u32_2 == 0 {
      return Err("Division by zero in u32 modulo".to_string());
    }
    match arc_u32_1.checked_rem(*arc_u32_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u32 modulo".to_string()),
    }
  }

  // Try u64 % u64
  if let (Ok(arc_u64_1), Ok(arc_u64_2)) =
    (v1.clone().downcast::<u64>(), v2.clone().downcast::<u64>())
  {
    if *arc_u64_2 == 0 {
      return Err("Division by zero in u64 modulo".to_string());
    }
    match arc_u64_1.checked_rem(*arc_u64_2) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u64 modulo".to_string()),
    }
  }

  // Try f32 % f32
  if let (Ok(arc_f32_1), Ok(arc_f32_2)) =
    (v1.clone().downcast::<f32>(), v2.clone().downcast::<f32>())
  {
    if *arc_f32_2 == 0.0 {
      return Err("Division by zero in f32 modulo".to_string());
    }
    return Ok(Arc::new(*arc_f32_1 % *arc_f32_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try f64 % f64
  if let (Ok(arc_f64_1), Ok(arc_f64_2)) =
    (v1.clone().downcast::<f64>(), v2.clone().downcast::<f64>())
  {
    if *arc_f64_2 == 0.0 {
      return Err("Division by zero in f64 modulo".to_string());
    }
    return Ok(Arc::new(*arc_f64_1 % *arc_f64_2) as Arc<dyn Any + Send + Sync>);
  }

  // Try type promotion: i32 % i64 -> i64
  if let (Ok(arc_i32), Ok(arc_i64)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<i64>()) {
    if *arc_i64 == 0 {
      return Err("Division by zero in i32 % i64 modulo".to_string());
    }
    match (*arc_i32 as i64).checked_rem(*arc_i64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i32 % i64 modulo".to_string()),
    }
  }
  if let (Ok(arc_i64), Ok(arc_i32)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<i32>()) {
    if *arc_i32 == 0 {
      return Err("Division by zero in i64 % i32 modulo".to_string());
    }
    match arc_i64.checked_rem(*arc_i32 as i64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in i64 % i32 modulo".to_string()),
    }
  }

  // Try type promotion: u32 % u64 -> u64
  if let (Ok(arc_u32), Ok(arc_u64)) = (v1.clone().downcast::<u32>(), v2.clone().downcast::<u64>()) {
    if *arc_u64 == 0 {
      return Err("Division by zero in u32 % u64 modulo".to_string());
    }
    match (*arc_u32 as u64).checked_rem(*arc_u64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u32 % u64 modulo".to_string()),
    }
  }
  if let (Ok(arc_u64), Ok(arc_u32)) = (v1.clone().downcast::<u64>(), v2.clone().downcast::<u32>()) {
    if *arc_u32 == 0 {
      return Err("Division by zero in u64 % u32 modulo".to_string());
    }
    match arc_u64.checked_rem(*arc_u32 as u64) {
      Some(result) => return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>),
      None => return Err("Integer overflow in u64 % u32 modulo".to_string()),
    }
  }

  // Try type promotion: integer % float -> float
  if let (Ok(arc_i32), Ok(arc_f32)) = (v1.clone().downcast::<i32>(), v2.clone().downcast::<f32>()) {
    if *arc_f32 == 0.0 {
      return Err("Division by zero in i32 % f32 modulo".to_string());
    }
    return Ok(Arc::new(*arc_i32 as f32 % *arc_f32) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f32), Ok(arc_i32)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<i32>()) {
    if *arc_i32 == 0 {
      return Err("Division by zero in f32 % i32 modulo".to_string());
    }
    return Ok(Arc::new(*arc_f32 % *arc_i32 as f32) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_i64), Ok(arc_f64)) = (v1.clone().downcast::<i64>(), v2.clone().downcast::<f64>()) {
    if *arc_f64 == 0.0 {
      return Err("Division by zero in i64 % f64 modulo".to_string());
    }
    return Ok(Arc::new(*arc_i64 as f64 % *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_i64)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<i64>()) {
    if *arc_i64 == 0 {
      return Err("Division by zero in f64 % i64 modulo".to_string());
    }
    return Ok(Arc::new(*arc_f64 % *arc_i64 as f64) as Arc<dyn Any + Send + Sync>);
  }

  if let (Ok(arc_f32), Ok(arc_f64)) = (v1.clone().downcast::<f32>(), v2.clone().downcast::<f64>()) {
    if *arc_f64 == 0.0 {
      return Err("Division by zero in f32 % f64 modulo".to_string());
    }
    return Ok(Arc::new(*arc_f32 as f64 % *arc_f64) as Arc<dyn Any + Send + Sync>);
  }
  if let (Ok(arc_f64), Ok(arc_f32)) = (v1.clone().downcast::<f64>(), v2.clone().downcast::<f32>()) {
    if *arc_f32 == 0.0 {
      return Err("Division by zero in f64 % f32 modulo".to_string());
    }
    return Ok(Arc::new(*arc_f64 % *arc_f32 as f64) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported types for modulo: {} % {}",
    std::any::type_name_of_val(&**v1),
    std::any::type_name_of_val(&**v2)
  ))
}
