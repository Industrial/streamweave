//! Common utility functions for object operations.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

/// Extracts keys from an object (HashMap).
///
/// This function attempts to downcast the object to its expected type
/// and extracts its keys. It supports:
/// - Extracting keys from HashMap<String, Arc<dyn Any + Send + Sync>> objects
/// - Returns array of key strings
/// - Preserving key order (HashMap iteration order)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Arc<dyn Any + Send + Sync>>) or an error string.
pub fn object_keys(v: &Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast object to HashMap
  let arc_map = v
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for object keys input: {} (input must be HashMap<String, Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Extract keys and convert to array
  let keys: Vec<Arc<dyn Any + Send + Sync>> = arc_map
    .keys()
    .map(|k| Arc::new(k.clone()) as Arc<dyn Any + Send + Sync>)
    .collect();

  Ok(Arc::new(keys) as Arc<dyn Any + Send + Sync>)
}

/// Extracts values from an object (HashMap).
///
/// This function attempts to downcast the object to its expected type
/// and extracts its values. It supports:
/// - Extracting values from HashMap<String, Arc<dyn Any + Send + Sync>> objects
/// - Returns array of values
/// - Preserving value order (HashMap iteration order)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Arc<dyn Any + Send + Sync>>) or an error string.
pub fn object_values(v: &Arc<dyn Any + Send + Sync>) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast object to HashMap
  let arc_map = v
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for object values input: {} (input must be HashMap<String, Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Extract values and convert to array
  let values: Vec<Arc<dyn Any + Send + Sync>> = arc_map.values().cloned().collect();

  Ok(Arc::new(values) as Arc<dyn Any + Send + Sync>)
}
