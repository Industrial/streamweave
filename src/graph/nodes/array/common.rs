//! Common utility functions for array operations.

use crate::graph::nodes::comparison::common::{compare_equal, compare_less_than};
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

/// Finds the index of a value in an array.
///
/// This function attempts to downcast the array to its expected type
/// and finds the index of the specified value. It supports:
/// - Finding index for any value type (uses compare_equal for type-aware comparison)
/// - Type promotion (e.g., i32 matches i64)
/// - Returns -1 if value is not found (similar to JavaScript's indexOf)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (i32 index, or -1 if not found) or an error string.
pub fn array_index_of(
  v: &Arc<dyn Any + Send + Sync>,
  value: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array index_of input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Iterate through array elements and find the index
  for (index, element) in arc_vec.iter().enumerate() {
    match compare_equal(element, value) {
      Ok(result_arc) => {
        // Downcast the boolean result
        if let Ok(arc_bool) = result_arc.downcast::<bool>()
          && *arc_bool
        {
          // Found the value, return the index as i32
          return Ok(Arc::new(index as i32) as Arc<dyn Any + Send + Sync>);
        }
      }
      Err(_) => {
        // If comparison fails, continue to next element
        continue;
      }
    }
  }

  // No match found, return -1
  Ok(Arc::new(-1i32) as Arc<dyn Any + Send + Sync>)
}

/// Concatenates two arrays.
///
/// This function attempts to downcast both arrays to their expected types
/// and concatenates them. It supports:
/// - Concatenating two Vec<Arc<dyn Any + Send + Sync>> arrays
/// - Preserving element order (first array, then second array)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Arc<dyn Any + Send + Sync>>) or an error string.
pub fn array_concat(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast first array
  let arc_vec1 = v1
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array concat first input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v1)
      )
    })?;

  // Try to downcast second array
  let arc_vec2 = v2
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array concat second input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v2)
      )
    })?;

  // Concatenate arrays (clone the Arc references)
  let mut result: Vec<Arc<dyn Any + Send + Sync>> =
    Vec::with_capacity(arc_vec1.len() + arc_vec2.len());
  result.extend(arc_vec1.iter().cloned());
  result.extend(arc_vec2.iter().cloned());

  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Reverses the order of elements in an array.
///
/// This function attempts to downcast the array to its expected type
/// and reverses the element order. It supports:
/// - Reversing Vec<Arc<dyn Any + Send + Sync>> arrays
/// - Preserving element references (clones Arc references)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Arc<dyn Any + Send + Sync>>) or an error string.
pub fn array_reverse(v: &Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array reverse input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Reverse the array (clone the Arc references)
  let mut result: Vec<Arc<dyn Any + Send + Sync>> = arc_vec.iter().cloned().collect();
  result.reverse();

  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Sorts an array of elements.
///
/// This function attempts to downcast the array to its expected type
/// and sorts the elements. It supports:
/// - Sorting Vec<Arc<dyn Any + Send + Sync>> arrays
/// - Ascending or descending order
/// - Type-aware comparison (uses compare_less_than for type promotion)
/// - Incompatible types are placed at the end (via compare_less_than returning false)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Arc<dyn Any + Send + Sync>>) or an error string.
pub fn array_sort(
  v: &Arc<dyn Any + Send + Sync>,
  ascending: bool,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array sort input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Create a mutable copy for sorting
  let mut result: Vec<Arc<dyn Any + Send + Sync>> = arc_vec.iter().cloned().collect();

  // Sort using compare_less_than (for numeric types) and direct comparison (for strings)
  result.sort_by(|a, b| {
    // Try string comparison first
    if let (Ok(arc_str1), Ok(arc_str2)) = (
      a.clone().downcast::<String>(),
      b.clone().downcast::<String>(),
    ) {
      return arc_str1.as_str().cmp(arc_str2.as_str());
    }

    // Try numeric comparison using compare_less_than
    match compare_less_than(a, b) {
      Ok(result_arc) => {
        if let Ok(arc_bool) = result_arc.downcast::<bool>()
          && *arc_bool
        {
          return std::cmp::Ordering::Less;
        }
        // Check if equal (if not less, check if greater)
        if let Ok(result_arc2) = compare_less_than(b, a)
          && let Ok(arc_bool2) = result_arc2.downcast::<bool>()
          && *arc_bool2
        {
          return std::cmp::Ordering::Greater;
        }
        std::cmp::Ordering::Equal
      }
      Err(_) => {
        // If comparison fails, consider them equal (incompatible types)
        std::cmp::Ordering::Equal
      }
    }
  });

  // Reverse if descending
  if !ascending {
    result.reverse();
  }

  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Filters an array based on a predicate function.
///
/// This function attempts to downcast the array to its expected type
/// and filters elements based on the predicate. It supports:
/// - Filtering Vec<Arc<dyn Any + Send + Sync>> arrays
/// - Predicate function that evaluates each element to a boolean
/// - Preserving element references (clones Arc references for kept elements)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Arc<dyn Any + Send + Sync>>) or an error string.
pub async fn array_filter(
  v: &Arc<dyn Any + Send + Sync>,
  predicate: &crate::graph::nodes::FilterConfig,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array filter input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Filter elements using the predicate
  let mut result: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  for element in arc_vec.iter() {
    match predicate.apply(element.clone()).await {
      Ok(true) => {
        // Element passes filter - keep it (zero-copy: clone Arc reference)
        result.push(element.clone());
      }
      Ok(false) => {
        // Element filtered out - skip it
      }
      Err(e) => {
        return Err(format!("Predicate error: {}", e));
      }
    }
  }

  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Maps an array by applying a transformation function to each element.
///
/// This function attempts to downcast the array to its expected type
/// and applies the transformation to each element. It supports:
/// - Mapping Vec<Arc<dyn Any + Send + Sync>> arrays
/// - Transformation function that transforms each element
/// - Preserving element order
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Arc<dyn Any + Send + Sync>>) or an error string.
pub async fn array_map(
  v: &Arc<dyn Any + Send + Sync>,
  map_fn: &crate::graph::nodes::MapConfig,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array map input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Map elements using the transformation function
  let mut result: Vec<Arc<dyn Any + Send + Sync>> = Vec::with_capacity(arc_vec.len());
  for element in arc_vec.iter() {
    match map_fn.apply(element.clone()).await {
      Ok(transformed) => {
        // Element transformed successfully - add to result
        result.push(transformed);
      }
      Err(e) => {
        return Err(format!("Map function error: {}", e));
      }
    }
  }

  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Joins an array of elements into a string using a delimiter.
///
/// This function attempts to downcast the array to its expected type
/// and joins elements with a delimiter. It supports:
/// - Joining Vec<Arc<dyn Any + Send + Sync>> arrays
/// - Converting each element to string representation
/// - Custom delimiter string
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (String) or an error string.
pub fn array_join(
  v: &Arc<dyn Any + Send + Sync>,
  delimiter: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array join input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Try to downcast delimiter
  let arc_delimiter = delimiter.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for delimiter: {} (delimiter must be String)",
      std::any::type_name_of_val(&**delimiter)
    )
  })?;

  // Convert each element to String and join
  let mut parts = Vec::new();
  for (idx, item) in arc_vec.iter().enumerate() {
    // Try to convert element to string
    let str_repr = if let Ok(arc_str) = item.clone().downcast::<String>() {
      (*arc_str).clone()
    } else if let Ok(arc_i32) = item.clone().downcast::<i32>() {
      arc_i32.to_string()
    } else if let Ok(arc_i64) = item.clone().downcast::<i64>() {
      arc_i64.to_string()
    } else if let Ok(arc_u32) = item.clone().downcast::<u32>() {
      arc_u32.to_string()
    } else if let Ok(arc_u64) = item.clone().downcast::<u64>() {
      arc_u64.to_string()
    } else if let Ok(arc_f32) = item.clone().downcast::<f32>() {
      arc_f32.to_string()
    } else if let Ok(arc_f64) = item.clone().downcast::<f64>() {
      arc_f64.to_string()
    } else if let Ok(arc_bool) = item.clone().downcast::<bool>() {
      arc_bool.to_string()
    } else {
      return Err(format!(
        "Unsupported type for array element at index {}: {} (element must be convertible to string)",
        idx,
        std::any::type_name_of_val(&**item)
      ));
    };
    parts.push(str_repr);
  }

  // Join with delimiter
  let result = parts.join(&*arc_delimiter);
  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Splits an array into chunks of a specified size.
///
/// This function attempts to downcast the array to its expected type
/// and splits it into chunks. It supports:
/// - Splitting Vec<Arc<dyn Any + Send + Sync>> arrays
/// - Configurable chunk size
/// - Preserving element references (clones Arc references)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Vec<Arc<dyn Any + Send + Sync>>>) or an error string.
pub fn array_split(
  v: &Arc<dyn Any + Send + Sync>,
  chunk_size: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array split input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Helper to convert chunk_size to usize
  let get_usize = |val: &Arc<dyn Any + Send + Sync>, name: &str| -> Result<usize, String> {
    if let Ok(arc_i32) = val.clone().downcast::<i32>() {
      let i = *arc_i32;
      if i < 0 {
        return Err(format!("{} must be non-negative, got {}", name, i));
      }
      Ok(i as usize)
    } else if let Ok(arc_i64) = val.clone().downcast::<i64>() {
      let i = *arc_i64;
      if i < 0 {
        return Err(format!("{} must be non-negative, got {}", name, i));
      }
      if i > usize::MAX as i64 {
        return Err(format!("{} overflow: {} exceeds usize::MAX", name, i));
      }
      Ok(i as usize)
    } else if let Ok(arc_u32) = val.clone().downcast::<u32>() {
      Ok(*arc_u32 as usize)
    } else if let Ok(arc_u64) = val.clone().downcast::<u64>() {
      let u = *arc_u64;
      if u > usize::MAX as u64 {
        return Err(format!("{} overflow: {} exceeds usize::MAX", name, u));
      }
      Ok(u as usize)
    } else {
      Err(format!(
        "Unsupported type for {}: {} (must be i32, i64, u32, or u64)",
        name,
        std::any::type_name_of_val(&**val)
      ))
    }
  };

  let size = get_usize(chunk_size, "chunk_size")?;
  if size == 0 {
    return Err("chunk_size must be greater than 0".to_string());
  }

  // Split into chunks
  let mut result: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  for chunk in arc_vec.chunks(size) {
    let chunk_vec: Vec<Arc<dyn Any + Send + Sync>> = chunk.to_vec();
    result.push(Arc::new(chunk_vec) as Arc<dyn Any + Send + Sync>);
  }

  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Flattens a nested array one level.
///
/// This function attempts to downcast the array to its expected type
/// and flattens nested arrays one level. It supports:
/// - Flattening Vec<Arc<dyn Any + Send + Sync>> arrays where elements are also arrays
/// - Flattening one level only (not recursive)
/// - Preserving element references (clones Arc references)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Arc<dyn Any + Send + Sync>>) or an error string.
pub fn array_flatten(v: &Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for array flatten input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Flatten one level: iterate through elements and if an element is an array, add its elements
  let mut result: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  for item in arc_vec.iter() {
    // Try to downcast element to array
    if let Ok(nested_array) = item.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
      // Element is an array - flatten it by adding its elements
      result.extend(nested_array.iter().cloned());
    } else {
      // Element is not an array - add it as-is
      result.push(item.clone());
    }
  }

  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}
