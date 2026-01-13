//! Common utility functions for array operations.

use std::any::Any;
use std::sync::Arc;

/// Gets the length of an array.
///
/// This function attempts to downcast the array to its expected type
/// and returns its length. It supports:
/// - Getting the length of Vec<Arc<dyn Any + Send + Sync>>
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (usize) or an error string.
pub fn array_length(v: &Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array length input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Return the length as usize
  let length = arc_vec.len();
  Ok(Arc::new(length) as Arc<dyn Any + Send + Sync>)
}
