use crate::traits::monoid::Monoid;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;
use std::hash::Hash;

impl<K, V> Monoid for HashMap<K, V>
where
  K: Eq + Hash + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn empty() -> Self {
    HashMap::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;

  #[derive(Clone, Debug, PartialEq, Eq, Hash)]
  struct TestKey(String);

  #[derive(Clone, Debug, PartialEq, Eq)]
  struct TestValue(i32);

  // Test that empty returns an empty HashMap
  #[test]
  fn test_empty() {
    let empty_map: HashMap<String, i32> = HashMap::empty();
    assert!(empty_map.is_empty());
    assert_eq!(empty_map.len(), 0);
  }

  // Test combining empty maps
  #[test]
  fn test_combine_empty() {
    let map1: HashMap<String, i32> = HashMap::empty();
    let map2: HashMap<String, i32> = HashMap::empty();

    // Empty + Empty = Empty
    let result = map1.clone().combine(map2.clone());
    assert!(result.is_empty());

    // Map + Empty = Map (right identity)
    let mut map3 = HashMap::new();
    map3.insert("a".to_string(), 1);
    map3.insert("b".to_string(), 2);

    let result = map3.clone().combine(HashMap::empty());
    assert_eq!(result, map3);

    // Empty + Map = Map (left identity)
    let result = HashMap::<String, i32>::empty().combine(map3.clone());
    assert_eq!(result, map3);
  }

  // Test combining maps with non-overlapping keys
  #[test]
  fn test_combine_non_overlapping() {
    let mut map1 = HashMap::new();
    map1.insert("a".to_string(), 1);
    map1.insert("b".to_string(), 2);

    let mut map2 = HashMap::new();
    map2.insert("c".to_string(), 3);
    map2.insert("d".to_string(), 4);

    let result = map1.clone().combine(map2.clone());

    assert_eq!(result.len(), 4);
    assert_eq!(result.get("a"), Some(&1));
    assert_eq!(result.get("b"), Some(&2));
    assert_eq!(result.get("c"), Some(&3));
    assert_eq!(result.get("d"), Some(&4));
  }

  // Test combining maps with overlapping keys (later values overwrite earlier ones)
  #[test]
  fn test_combine_overlapping() {
    let mut map1 = HashMap::new();
    map1.insert("a".to_string(), 1);
    map1.insert("b".to_string(), 2);

    let mut map2 = HashMap::new();
    map2.insert("b".to_string(), 20);
    map2.insert("c".to_string(), 30);

    let result = map1.clone().combine(map2.clone());

    assert_eq!(result.len(), 3);
    assert_eq!(result.get("a"), Some(&1));
    assert_eq!(result.get("b"), Some(&20)); // Value from map2 overwrites map1
    assert_eq!(result.get("c"), Some(&30));
  }

  // Test combining maps with custom types
  #[test]
  fn test_combine_custom_types() {
    let mut map1 = HashMap::new();
    map1.insert(TestKey("a".to_string()), TestValue(1));
    map1.insert(TestKey("b".to_string()), TestValue(2));

    let mut map2 = HashMap::new();
    map2.insert(TestKey("c".to_string()), TestValue(3));

    let result = map1.clone().combine(map2.clone());

    assert_eq!(result.len(), 3);
    assert_eq!(result.get(&TestKey("a".to_string())), Some(&TestValue(1)));
    assert_eq!(result.get(&TestKey("b".to_string())), Some(&TestValue(2)));
    assert_eq!(result.get(&TestKey("c".to_string())), Some(&TestValue(3)));
  }

  // Test monoid left identity law: empty().combine(x) == x
  #[test]
  fn test_left_identity() {
    let mut map = HashMap::new();
    map.insert("a".to_string(), 1);
    map.insert("b".to_string(), 2);

    let result = HashMap::<String, i32>::empty().combine(map.clone());
    assert_eq!(result, map);
  }

  // Test monoid right identity law: x.combine(empty()) == x
  #[test]
  fn test_right_identity() {
    let mut map = HashMap::new();
    map.insert("a".to_string(), 1);
    map.insert("b".to_string(), 2);

    let result = map.clone().combine(HashMap::empty());
    assert_eq!(result, map);
  }

  // Test monoid associativity: (a.combine(b)).combine(c) == a.combine(b.combine(c))
  #[test]
  fn test_associativity() {
    let mut map1 = HashMap::new();
    map1.insert("a".to_string(), 1);

    let mut map2 = HashMap::new();
    map2.insert("b".to_string(), 2);

    let mut map3 = HashMap::new();
    map3.insert("c".to_string(), 3);

    // (map1 + map2) + map3
    let result1 = map1.clone().combine(map2.clone()).combine(map3.clone());

    // map1 + (map2 + map3)
    let result2 = map1.clone().combine(map2.clone().combine(map3.clone()));

    assert_eq!(result1, result2);
  }

  // Test combining with overlapping keys
  #[test]
  fn test_combine_precedence() {
    let mut map1 = HashMap::new();
    map1.insert("key".to_string(), "first".to_string());

    let mut map2 = HashMap::new();
    map2.insert("key".to_string(), "second".to_string());

    let result = map1.combine(map2);
    assert_eq!(result.get("key"), Some(&"second".to_string()));
  }

  // Property-based test for left identity law
  #[test]
  fn prop_left_identity() {
    let mut map = HashMap::new();
    map.insert("a".to_string(), 1);
    map.insert("b".to_string(), 2);
    map.insert("c".to_string(), 3);

    let result = HashMap::<String, i32>::empty().combine(map.clone());
    assert_eq!(result, map);
  }

  // Property-based test for right identity law
  #[test]
  fn prop_right_identity() {
    let mut map = HashMap::new();
    map.insert("a".to_string(), 1);
    map.insert("b".to_string(), 2);
    map.insert("c".to_string(), 3);

    let result = map.clone().combine(HashMap::empty());
    assert_eq!(result, map);
  }

  // Property-based test for associativity
  #[test]
  fn prop_associativity() {
    let mut map1 = HashMap::new();
    map1.insert("a".to_string(), 1);
    map1.insert("common".to_string(), 10);

    let mut map2 = HashMap::new();
    map2.insert("b".to_string(), 2);
    map2.insert("common".to_string(), 20);

    let mut map3 = HashMap::new();
    map3.insert("c".to_string(), 3);
    map3.insert("common".to_string(), 30);

    // (map1 + map2) + map3
    let result1 = map1.clone().combine(map2.clone()).combine(map3.clone());

    // map1 + (map2 + map3)
    let result2 = map1.clone().combine(map2.clone().combine(map3.clone()));

    assert_eq!(result1, result2);

    // Check that the rightmost value for the common key wins
    assert_eq!(result1.get("common"), Some(&30));
  }

  // Test mconcat with multiple maps
  #[test]
  fn test_mconcat() {
    let mut map1 = HashMap::new();
    map1.insert("a".to_string(), 1);

    let mut map2 = HashMap::new();
    map2.insert("b".to_string(), 2);

    let mut map3 = HashMap::new();
    map3.insert("c".to_string(), 3);

    let maps = vec![map1, map2, map3];
    let result = Monoid::mconcat(maps);

    assert_eq!(result.len(), 3);
    assert_eq!(result.get("a"), Some(&1));
    assert_eq!(result.get("b"), Some(&2));
    assert_eq!(result.get("c"), Some(&3));
  }

  // Test mconcat with empty vector
  #[test]
  fn test_mconcat_empty() {
    let maps: Vec<HashMap<String, i32>> = vec![];
    let result = Monoid::mconcat(maps);

    assert!(result.is_empty());
  }

  // Test mconcat with overlapping keys (later values should win)
  #[test]
  fn test_mconcat_overlapping() {
    let mut map1 = HashMap::new();
    map1.insert("common".to_string(), 1);
    map1.insert("a".to_string(), 10);

    let mut map2 = HashMap::new();
    map2.insert("common".to_string(), 2);
    map2.insert("b".to_string(), 20);

    let mut map3 = HashMap::new();
    map3.insert("common".to_string(), 3);
    map3.insert("c".to_string(), 30);

    let maps = vec![map1, map2, map3];
    let result = Monoid::mconcat(maps);

    assert_eq!(result.len(), 4);
    assert_eq!(result.get("a"), Some(&10));
    assert_eq!(result.get("b"), Some(&20));
    assert_eq!(result.get("c"), Some(&30));
    assert_eq!(result.get("common"), Some(&3)); // Last value wins
  }

  // Test mconcat with a single map
  #[test]
  fn test_mconcat_single() {
    let mut map = HashMap::new();
    map.insert("a".to_string(), 1);
    map.insert("b".to_string(), 2);

    let maps = vec![map.clone()];
    let result = Monoid::mconcat(maps);

    assert_eq!(result, map);
  }

  // Test mconcat with custom types
  #[test]
  fn test_mconcat_custom_types() {
    let mut map1 = HashMap::new();
    map1.insert(TestKey("a".to_string()), TestValue(1));

    let mut map2 = HashMap::new();
    map2.insert(TestKey("b".to_string()), TestValue(2));

    let maps = vec![map1, map2];
    let result = Monoid::mconcat(maps);

    assert_eq!(result.len(), 2);
    assert_eq!(result.get(&TestKey("a".to_string())), Some(&TestValue(1)));
    assert_eq!(result.get(&TestKey("b".to_string())), Some(&TestValue(2)));
  }

  // Test with a large number of maps
  #[test]
  fn test_mconcat_large() {
    let mut maps = Vec::new();

    for i in 0..10 {
      let mut map = HashMap::new();
      map.insert(format!("key{}", i), i);
      maps.push(map);
    }

    let result = Monoid::mconcat(maps);
    assert_eq!(result.len(), 10);

    for i in 0..10 {
      assert_eq!(result.get(&format!("key{}", i)), Some(&i));
    }
  }
}
