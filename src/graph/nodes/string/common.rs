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

/// Splits a string by a delimiter and returns a vector of strings.
///
/// This function attempts to downcast the string and delimiter to their
/// expected types and performs splitting. It supports:
/// - String splitting by literal delimiter
/// - Returns Vec<Arc<dyn Any + Send + Sync>> where each element is a String
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (wrapping Vec) or an error string.
pub fn string_split(
  v: &Arc<dyn Any + Send + Sync>,
  delimiter: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast string
  let arc_str = v.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string split input: {} (input must be String)",
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

  // Perform split
  let parts: Vec<Arc<dyn Any + Send + Sync>> = arc_str
    .split(&*arc_delimiter)
    .map(|s| Arc::new(s.to_string()) as Arc<dyn Any + Send + Sync>)
    .collect();

  Ok(Arc::new(parts) as Arc<dyn Any + Send + Sync>)
}

/// Joins an array of strings with a delimiter.
///
/// This function attempts to downcast the array and delimiter to their
/// expected types and performs joining. It supports:
/// - Joining Vec<Arc<dyn Any + Send + Sync>> where each element is a String
/// - Returns a single joined string
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn string_join(
  v: &Arc<dyn Any + Send + Sync>,
  delimiter: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast array
  let arc_vec = v
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for string join input: {} (input must be Vec<Arc<dyn Any + Send + Sync>>)",
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
    let arc_str = item.clone().downcast::<String>().map_err(|_| {
      format!(
        "Unsupported type for array element at index {}: {} (all elements must be String)",
        idx,
        std::any::type_name_of_val(&**item)
      )
    })?;
    parts.push((*arc_str).clone());
  }

  // Join with delimiter
  let result = parts.join(&*arc_delimiter);
  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Checks if a string contains a substring.
///
/// This function attempts to downcast the string and substring to their
/// expected types and performs the contains check. It supports:
/// - Case-sensitive substring matching
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (bool) or an error string.
pub fn string_contains(
  v: &Arc<dyn Any + Send + Sync>,
  substring: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast string
  let arc_str = v.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string contains input: {} (input must be String)",
      std::any::type_name_of_val(&**v)
    )
  })?;

  // Try to downcast substring
  let arc_substring = substring.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for substring: {} (substring must be String)",
      std::any::type_name_of_val(&**substring)
    )
  })?;

  // Perform contains check (case-sensitive)
  let result = arc_str.contains(&*arc_substring);
  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Checks if a string starts with a prefix.
///
/// This function attempts to downcast the string and prefix to their
/// expected types and performs the starts_with check. It supports:
/// - Case-sensitive prefix matching
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (bool) or an error string.
pub fn string_starts_with(
  v: &Arc<dyn Any + Send + Sync>,
  prefix: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast string
  let arc_str = v.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string starts_with input: {} (input must be String)",
      std::any::type_name_of_val(&**v)
    )
  })?;

  // Try to downcast prefix
  let arc_prefix = prefix.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for prefix: {} (prefix must be String)",
      std::any::type_name_of_val(&**prefix)
    )
  })?;

  // Perform starts_with check (case-sensitive)
  let result = arc_str.starts_with(&*arc_prefix);
  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}
