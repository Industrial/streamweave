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

/// Gets the length of a string value.
///
/// This function attempts to downcast the value to String and returns
/// its character count as a usize.
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn string_length(v: &Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try String
  if let Ok(arc_str) = v.clone().downcast::<String>() {
    let length = arc_str.len();
    return Ok(Arc::new(length) as Arc<dyn Any + Send + Sync>);
  }

  Err(format!(
    "Unsupported type for string length: {} (input must be String)",
    std::any::type_name_of_val(&**v)
  ))
}

/// Extracts a substring from a string using start and end indices.
///
/// This function attempts to downcast the string and indices to their expected types
/// and extracts the substring. It handles:
/// - String slicing with usize indices
/// - Bounds checking
/// - Invalid index ranges (start > end, out of bounds)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn string_slice(
  v: &Arc<dyn Any + Send + Sync>,
  start: &Arc<dyn Any + Send + Sync>,
  end: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast string
  let arc_str = v.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string slice input: {} (input must be String)",
      std::any::type_name_of_val(&**v)
    )
  })?;

  // Try to downcast start index (usize, i32, i64, u32, u64)
  let start_idx = if let Ok(arc_usize) = start.clone().downcast::<usize>() {
    *arc_usize
  } else if let Ok(arc_i32) = start.clone().downcast::<i32>() {
    if *arc_i32 < 0 {
      return Err("Start index cannot be negative".to_string());
    }
    *arc_i32 as usize
  } else if let Ok(arc_i64) = start.clone().downcast::<i64>() {
    if *arc_i64 < 0 {
      return Err("Start index cannot be negative".to_string());
    }
    if *arc_i64 > usize::MAX as i64 {
      return Err("Start index too large".to_string());
    }
    *arc_i64 as usize
  } else if let Ok(arc_u32) = start.clone().downcast::<u32>() {
    *arc_u32 as usize
  } else if let Ok(arc_u64) = start.clone().downcast::<u64>() {
    if *arc_u64 > usize::MAX as u64 {
      return Err("Start index too large".to_string());
    }
    *arc_u64 as usize
  } else {
    return Err(format!(
      "Unsupported type for start index: {} (must be numeric)",
      std::any::type_name_of_val(&**start)
    ));
  };

  // Try to downcast end index (usize, i32, i64, u32, u64)
  let end_idx = if let Ok(arc_usize) = end.clone().downcast::<usize>() {
    *arc_usize
  } else if let Ok(arc_i32) = end.clone().downcast::<i32>() {
    if *arc_i32 < 0 {
      return Err("End index cannot be negative".to_string());
    }
    *arc_i32 as usize
  } else if let Ok(arc_i64) = end.clone().downcast::<i64>() {
    if *arc_i64 < 0 {
      return Err("End index cannot be negative".to_string());
    }
    if *arc_i64 > usize::MAX as i64 {
      return Err("End index too large".to_string());
    }
    *arc_i64 as usize
  } else if let Ok(arc_u32) = end.clone().downcast::<u32>() {
    *arc_u32 as usize
  } else if let Ok(arc_u64) = end.clone().downcast::<u64>() {
    if *arc_u64 > usize::MAX as u64 {
      return Err("End index too large".to_string());
    }
    *arc_u64 as usize
  } else {
    return Err(format!(
      "Unsupported type for end index: {} (must be numeric)",
      std::any::type_name_of_val(&**end)
    ));
  };

  // Validate indices
  if start_idx > end_idx {
    return Err(format!(
      "Invalid index range: start ({}) > end ({})",
      start_idx, end_idx
    ));
  }

  let str_len = arc_str.len();
  if start_idx > str_len {
    return Err(format!(
      "Start index ({}) out of bounds for string of length {}",
      start_idx, str_len
    ));
  }

  if end_idx > str_len {
    return Err(format!(
      "End index ({}) out of bounds for string of length {}",
      end_idx, str_len
    ));
  }

  // Extract substring
  let result = arc_str[start_idx..end_idx].to_string();
  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Replaces all occurrences of a pattern in a string with a replacement string.
///
/// This function attempts to downcast the string, pattern, and replacement to their
/// expected types and performs replacement. It supports:
/// - String replacement with literal patterns
/// - All occurrences replacement
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn string_replace(
  v: &Arc<dyn Any + Send + Sync>,
  pattern: &Arc<dyn Any + Send + Sync>,
  replacement: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast string
  let arc_str = v.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string replace input: {} (input must be String)",
      std::any::type_name_of_val(&**v)
    )
  })?;

  // Try to downcast pattern
  let arc_pattern = pattern.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for pattern: {} (pattern must be String)",
      std::any::type_name_of_val(&**pattern)
    )
  })?;

  // Try to downcast replacement
  let arc_replacement = replacement.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for replacement: {} (replacement must be String)",
      std::any::type_name_of_val(&**replacement)
    )
  })?;

  // Perform replacement (replace all occurrences)
  let result = arc_str.replace(&*arc_pattern, &arc_replacement);
  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}
