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

/// Gets an element from an array by index.
///
/// This function attempts to downcast the array and index to their expected types
/// and returns the element at the specified index. It supports:
/// - Accessing elements by numeric index (usize, i32, i64, u32, u64)
/// - Handling out-of-bounds indices with errors
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn array_index(
  v: &Arc<dyn Any + Send + Sync>,
  index: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array index input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Helper to convert Arc<dyn Any> to usize, handling various numeric types
  let get_usize = |val: &Arc<dyn Any + Send + Sync>, name: &str| -> Result<usize, String> {
    if let Ok(arc_usize) = val.clone().downcast::<usize>() {
      Ok(*arc_usize)
    } else if let Ok(arc_i32) = val.clone().downcast::<i32>() {
      if *arc_i32 < 0 {
        return Err(format!("{} cannot be negative", name));
      }
      (*arc_i32)
        .try_into()
        .map_err(|_| format!("{} too large", name))
    } else if let Ok(arc_i64) = val.clone().downcast::<i64>() {
      if *arc_i64 < 0 {
        return Err(format!("{} cannot be negative", name));
      }
      (*arc_i64)
        .try_into()
        .map_err(|_| format!("{} too large", name))
    } else if let Ok(arc_u32) = val.clone().downcast::<u32>() {
      (*arc_u32)
        .try_into()
        .map_err(|_| format!("{} too large", name))
    } else if let Ok(arc_u64) = val.clone().downcast::<u64>() {
      (*arc_u64)
        .try_into()
        .map_err(|_| format!("{} too large", name))
    } else {
      Err(format!(
        "Unsupported type for {}: {} (must be numeric)",
        name,
        std::any::type_name_of_val(&**val)
      ))
    }
  };

  let idx = get_usize(index, "index")?;

  // Check bounds
  if idx >= arc_vec.len() {
    return Err(format!(
      "Index {} out of bounds for array of length {}",
      idx,
      arc_vec.len()
    ));
  }

  // Return the element at the index (clone the Arc)
  Ok(arc_vec[idx].clone())
}
