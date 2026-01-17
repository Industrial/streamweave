//! # Boolean Logic Common Utilities
//!
//! Shared utilities for boolean logic nodes, including type conversion functions.

use std::any::Any;
use std::sync::Arc;

/// Converts a value to a boolean, supporting common types that can be evaluated as booleans.
///
/// This function attempts to convert various types to boolean values:
/// - `bool`: Returns the value directly
/// - `i32`, `i64`: Returns `true` if non-zero, `false` if zero
/// - `String`: Returns `true` if non-empty, `false` if empty
///
/// # Arguments
///
/// * `value` - The value to convert, wrapped in `Arc<dyn Any + Send + Sync>`
///
/// # Returns
///
/// `Ok(bool)` if conversion succeeds, or `Err(String)` with an error message if the type
/// cannot be converted to boolean.
///
/// # Examples
///
/// ```rust,no_run
/// use streamweave::nodes::boolean_logic::common::to_bool;
/// use std::sync::Arc;
/// use std::any::Any;
///
/// // Convert boolean
/// let bool_val = Arc::new(true) as Arc<dyn Any + Send + Sync>;
/// assert_eq!(to_bool(&bool_val).unwrap(), true);
///
/// // Convert integer
/// let int_val = Arc::new(5i32) as Arc<dyn Any + Send + Sync>;
/// assert_eq!(to_bool(&int_val).unwrap(), true);
///
/// // Convert string
/// let str_val = Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>;
/// assert_eq!(to_bool(&str_val).unwrap(), true);
/// ```
/// Zero-copy boolean conversion - only clones Arc references, never the data.
pub fn to_bool(value: &Arc<dyn Any + Send + Sync>) -> Result<bool, String> {
  // Zero-copy: downcast consumes the Arc, so we clone the Arc reference (atomic refcount increment)
  // This is necessary because downcast takes ownership, but we only want to inspect the value
  // Try to downcast to boolean
  if let Ok(arc_bool) = value.clone().downcast::<bool>() {
    return Ok(*arc_bool);
  }

  // Try to downcast to integer (0 = false, non-zero = true)
  if let Ok(arc_i32) = value.clone().downcast::<i32>() {
    return Ok(*arc_i32 != 0);
  }
  if let Ok(arc_i64) = value.clone().downcast::<i64>() {
    return Ok(*arc_i64 != 0);
  }

  // Try to downcast to string (empty = false, non-empty = true)
  if let Ok(arc_str) = value.clone().downcast::<String>() {
    return Ok(!arc_str.is_empty());
  }

  Err(format!("Cannot convert to boolean: {:?}", value.type_id()))
}
