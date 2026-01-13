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

/// Extracts key-value pairs (entries) from an object (HashMap).
///
/// This function attempts to downcast the object to its expected type
/// and extracts its entries. It supports:
/// - Extracting entries from HashMap<String, Arc<dyn Any + Send + Sync>> objects
/// - Returns array of [key, value] pairs (each pair is an array with two elements)
/// - Preserving entry order (HashMap iteration order)
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (Vec<Arc<dyn Any + Send + Sync>>) or an error string.
pub fn object_entries(
  v: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast object to HashMap
  let arc_map = v
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for object entries input: {} (input must be HashMap<String, Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Extract entries and convert to array of [key, value] pairs
  let entries: Vec<Arc<dyn Any + Send + Sync>> = arc_map
    .iter()
    .map(|(k, v)| {
      // Each entry is an array [key, value]
      let pair: Vec<Arc<dyn Any + Send + Sync>> =
        vec![Arc::new(k.clone()) as Arc<dyn Any + Send + Sync>, v.clone()];
      Arc::new(pair) as Arc<dyn Any + Send + Sync>
    })
    .collect();

  Ok(Arc::new(entries) as Arc<dyn Any + Send + Sync>)
}

/// Gets a property value from an object (HashMap) by key.
///
/// This function attempts to downcast the object to its expected type
/// and gets the value for the specified key. It supports:
/// - Getting values from HashMap<String, Arc<dyn Any + Send + Sync>> objects
/// - Key must be a String
/// - Returns the value if key exists, or an error if key is missing
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (the property value) or an error string.
pub fn object_property(
  v: &Arc<dyn Any + Send + Sync>,
  key: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast object to HashMap
  let arc_map = v
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for object property input: {} (input must be HashMap<String, Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Try to downcast key to String
  let arc_key = key.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for object property key: {} (key must be String)",
      std::any::type_name_of_val(&**key)
    )
  })?;

  // Get the value for the key
  arc_map
    .get(&*arc_key)
    .cloned()
    .ok_or_else(|| format!("Property '{}' not found in object", *arc_key))
}

/// Checks if an object (HashMap) has a property with the specified key.
///
/// This function attempts to downcast the object to its expected type
/// and checks if it contains the specified key. It supports:
/// - Checking keys in HashMap<String, Arc<dyn Any + Send + Sync>> objects
/// - Key must be a String
/// - Returns true if key exists, false otherwise
///
/// Returns the result as `Arc<dyn Any + Send + Sync>` (boolean) or an error string.
pub fn object_has_property(
  v: &Arc<dyn Any + Send + Sync>,
  key: &Arc<dyn Any + Send + Sync>,
) -> Result<Arc<dyn Any + Send + Sync>, String> {
  // Try to downcast object to HashMap
  let arc_map = v
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
    .map_err(|_| {
      format!(
        "Unsupported type for object has_property input: {} (input must be HashMap<String, Arc<dyn Any + Send + Sync>>)",
        std::any::type_name_of_val(&**v)
      )
    })?;

  // Try to downcast key to String
  let arc_key = key.clone().downcast::<String>().map_err(|_| {
    format!(
      "Unsupported type for object has_property key: {} (key must be String)",
      std::any::type_name_of_val(&**key)
    )
  })?;

  // Check if the key exists
  let has_property = arc_map.contains_key(&*arc_key);
  Ok(Arc::new(has_property) as Arc<dyn Any + Send + Sync>)
}
