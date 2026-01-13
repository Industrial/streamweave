//! Common utility functions for array operations.

use crate::graph::nodes::comparison::common::compare_equal;
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

/// Extracts a slice from an array using start and end indices.
///
/// This function attempts to downcast the array and indices to their expected types
/// and returns a new array containing the sliced elements. It supports:
/// - Extracting slices with various numeric index types (usize, i32, i64, u32, u64)
/// - Handling out-of-bounds indices (clamps to array bounds)
/// - Invalid index range detection (start > end)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Arc<dyn Any + Send + Sync>>) or an error string.
pub fn array_slice(
  v: &Arc<dyn Any + Send + Sync>,
  start: &Arc<dyn Any + Send + Sync>,
  end: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array slice input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
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

  let start_idx = get_usize(start, "start")?;
  let end_idx = get_usize(end, "end")?;

  // Validate range
  if start_idx > end_idx {
    return Err(format!(
      "Invalid slice range: start index {} is greater than end index {}",
      start_idx, end_idx
    ));
  }

  let array_len = arc_vec.len();

  // Clamp indices to array bounds
  let clamped_start = start_idx.min(array_len);
  let clamped_end = end_idx.min(array_len);

  // Extract slice (clone the Arc references)
  let result: Vec<Arc<dyn Any + Send + Sync>> = arc_vec[clamped_start..clamped_end].to_vec();

  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Checks if an array contains a value.
///
/// This function attempts to downcast the array to its expected type
/// and checks if it contains the specified value. It supports:
/// - Checking for any value type (uses compare_equal for type-aware comparison)
/// - Type promotion (e.g., i32 matches i64)
/// - Returns false for incompatible types (via compare_equal)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (boolean) or an error string.
pub fn array_contains(
  v: &Arc<dyn Any + Send + Sync>,
  value: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array contains input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Iterate through array elements and check for equality
  for element in arc_vec.iter() {
    match compare_equal(element, value) {
      Ok(result_arc) => {
        // Downcast the boolean result
        if let Ok(arc_bool) = result_arc.downcast::<bool>()
          && *arc_bool
        {
          return Ok(Arc::new(true) as Arc<dyn Any + Send + Sync>);
        }
      }
      Err(_) => {
        // If comparison fails, continue to next element
        // (compare_equal returns false for incompatible types, so this shouldn't happen)
        continue;
      }
    }
  }

  // No match found
  Ok(Arc::new(false) as Arc<dyn Any + Send + Sync>)
}
