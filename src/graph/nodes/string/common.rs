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

/// Checks if a string ends with a suffix.
///
/// This function attempts to downcast the string and suffix to their
/// expected types and performs the ends_with check. It supports:
/// - Case-sensitive suffix matching
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (bool) or an error string.
pub fn string_ends_with(
  v: &Arc<dyn Any + Send + Sync>,
  suffix: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast string
  let arc_str = v.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string ends_with input: {} (input must be String)",
      std::any::type_name_of_val(&**v)
    )
  })?;

  // Try to downcast suffix
  let arc_suffix = suffix.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for suffix: {} (suffix must be String)",
      std::any::type_name_of_val(&**suffix)
    )
  })?;

  // Perform ends_with check (case-sensitive)
  let result = arc_str.ends_with(&*arc_suffix);
  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Checks if a string matches a regex pattern.
///
/// This function attempts to downcast the string and pattern to their
/// expected types and performs regex matching. It supports:
/// - Regex pattern matching (compiled at runtime)
/// - Returns boolean (true if matches, false otherwise)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (bool) or an error string.
pub fn string_match(
  v: &Arc<dyn Any + Send + Sync>,
  pattern: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast string
  let arc_str = v.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string match input: {} (input must be String)",
      std::any::type_name_of_val(&**v)
    )
  })?;

  // Try to downcast pattern
  let arc_pattern_str = pattern.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for pattern: {} (pattern must be String)",
      std::any::type_name_of_val(&**pattern)
    )
  })?;

  // Compile regex pattern
  let regex_pattern =
    regex::Regex::new(&arc_pattern_str).map_err(|e| format!("Invalid regex pattern: {}", e))?;

  // Perform regex match
  let result = regex_pattern.is_match(&arc_str);
  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Checks if two strings are equal.
///
/// This function attempts to downcast both strings to their
/// expected types and performs equality comparison. It supports:
/// - Case-sensitive string equality
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (bool) or an error string.
pub fn string_equal(
  v1: &Arc<dyn Any + Send + Sync>,
  v2: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast first string
  let arc_str1 = v1.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string equal input 1: {} (input must be String)",
      std::any::type_name_of_val(&**v1)
    )
  })?;

  // Try to downcast second string
  let arc_str2 = v2.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string equal input 2: {} (input must be String)",
      std::any::type_name_of_val(&**v2)
    )
  })?;

  // Perform equality check (case-sensitive)
  let result = *arc_str1 == *arc_str2;
  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Converts a string to a different case (upper, lower, or title).
///
/// This function attempts to downcast the string and case type to their
/// expected types and performs case conversion. It supports:
/// - "upper" or "uppercase": Converts to uppercase
/// - "lower" or "lowercase": Converts to lowercase
/// - "title" or "titlecase": Converts to title case (first letter of each word uppercase)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn string_case(
  v: &Arc<dyn Any + Send + Sync>,
  case_type: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast string
  let arc_str = v.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string case input: {} (input must be String)",
      std::any::type_name_of_val(&**v)
    )
  })?;

  // Try to downcast case type
  let arc_case_type = case_type.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for case type: {} (case type must be String)",
      std::any::type_name_of_val(&**case_type)
    )
  })?;

  // Perform case conversion
  let result = match arc_case_type.to_lowercase().as_str() {
    "upper" | "uppercase" => arc_str.to_uppercase(),
    "lower" | "lowercase" => arc_str.to_lowercase(),
    "title" | "titlecase" => {
      // Title case: first letter of each word uppercase, rest lowercase
      arc_str
        .split_whitespace()
        .map(|word| {
          let mut chars = word.chars();
          match chars.next() {
            None => String::new(),
            Some(first) => {
              first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
            }
          }
        })
        .collect::<Vec<_>>()
        .join(" ")
    }
    _ => {
      return Err(format!(
        "Unsupported case type: {} (must be 'upper', 'lower', or 'title')",
        *arc_case_type
      ));
    }
  };

  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Trims leading and trailing whitespace from a string.
///
/// This function attempts to downcast the string to its expected type
/// and performs trimming. It supports:
/// - Trimming leading and trailing whitespace
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn string_trim(v: &Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast string
  let arc_str = v.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string trim input: {} (input must be String)",
      std::any::type_name_of_val(&**v)
    )
  })?;

  // Perform trim
  let result = arc_str.trim().to_string();
  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Pads a string to a specified length with a padding character.
///
/// This function attempts to downcast the string, length, padding character, and side
/// to their expected types and performs padding. It supports:
/// - Left padding: Pads on the left side
/// - Right padding: Pads on the right side
/// - Center padding: Pads on both sides (preferring left if odd)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn string_pad(
  v: &Arc<dyn Any + Send + Sync>,
  length: &Arc<dyn Any + Send + Sync>,
  padding: &Arc<dyn Any + Send + Sync>,
  side: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast string
  let arc_str = v.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string pad input: {} (input must be String)",
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

  let target_length = get_usize(length, "length")?;

  // Try to downcast padding character (default to space if not provided or empty)
  let padding_char = if let Ok(arc_pad_str) = padding.clone().downcast::<String>() {
    let pad_str = &*arc_pad_str;
    if pad_str.is_empty() {
      ' '
    } else {
      pad_str.chars().next().unwrap_or(' ')
    }
  } else if let Ok(arc_char) = padding.clone().downcast::<char>() {
    *arc_char
  } else {
    ' ' // Default to space
  };

  // Try to downcast side (default to "right" if not provided)
  let side_str = if let Ok(arc_side) = side.clone().downcast::<String>() {
    arc_side.to_lowercase()
  } else {
    "right".to_string() // Default to right padding
  };

  let current_len = arc_str.chars().count();
  if current_len >= target_length {
    // String is already long enough, return as-is
    return Ok(arc_str.clone());
  }

  let pad_count = target_length - current_len;
  let pad_str: String = std::iter::repeat_n(padding_char, pad_count).collect();

  let result = match side_str.as_str() {
    "left" => format!("{}{}", pad_str, arc_str),
    "right" => format!("{}{}", arc_str, pad_str),
    "center" => {
      let left_pad = pad_count / 2;
      let right_pad = pad_count - left_pad;
      let left_str: String = std::iter::repeat_n(padding_char, left_pad).collect();
      let right_str: String = std::iter::repeat_n(padding_char, right_pad).collect();
      format!("{}{}{}", left_str, arc_str, right_str)
    }
    _ => {
      return Err(format!(
        "Unsupported padding side: {} (must be 'left', 'right', or 'center')",
        side_str
      ));
    }
  };

  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}

/// Reverses the character order of a string.
///
/// This function attempts to downcast the string to its expected type
/// and performs character reversal. It supports:
/// - Reversing the character order of a string
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` or an error string.
pub fn string_reverse(
  v: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast string
  let arc_str = v.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for string reverse input: {} (input must be String)",
      std::any::type_name_of_val(&**v)
    )
  })?;

  // Reverse the string by collecting chars in reverse order
  let result: String = arc_str.chars().rev().collect();
  Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
}
