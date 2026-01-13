//! # String Common Utilities
//!
//! Shared utilities for string operation nodes.

use std::any::Any;
use std::sync::Arc;

/// Concatenates two string values.
///
/// This function attempts to downcast both values to String and performs
/// concatenation. It handles:
/// - String + String: Direct concatenation
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn concat_strings(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try String + String
  if let (Ok(arc_str1), Ok(arc_str2)) = (
    v1.clone().downcast::<String>(),
    v2.clone().downcast::<String>(),
  ) {
    let result = format!("{}{}", *arc_str1, *arc_str2);
    return Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported types for string concatenation: {} + {} (both inputs must be String)",
    std::any::type_name_of_val(&**v1),
    std::any::type_name_of_val(&**v2)
  ))
}
