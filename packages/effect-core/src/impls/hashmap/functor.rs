use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;
use std::hash::Hash;

impl<K, A> Functor<A> for HashMap<K, A>
where
  K: Eq + Hash + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  type HigherSelf<U: CloneableThreadSafe> = HashMap<K, U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a A) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    self.into_iter().map(|(k, v)| (k, f(&v))).collect()
  }

  fn map_owned<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(A) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
    Self: Sized,
  {
    self.into_iter().map(|(k, v)| (k, f(v))).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashMap;

  // Test basic map functionality
  #[test]
  fn test_map_basic() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let result = map.map(|x| x * 2);

    let mut expected = HashMap::new();
    expected.insert("a", 2);
    expected.insert("b", 4);
    expected.insert("c", 6);

    assert_eq!(result, expected);
  }

  // Test functor identity law: map(x => x) == self
  #[test]
  fn test_functor_identity_law() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let clone = map.clone();
    let result = map.map(|x| *x);

    assert_eq!(result, clone);
  }

  // Test functor composition law: map(x => g(f(x))) == map(f).map(g)
  #[test]
  fn test_functor_composition_law() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    // First function: multiply by 2
    let f = |x: &i32| x * 2;

    // Second function: add 1
    let g = |x: &i32| x + 1;

    // Composition: map(x => g(f(x)))
    let result1 = map.clone().map(move |x| g(&f(x)));

    // Sequence: map(f).map(g)
    let result2 = map.clone().map(f).map(g);

    assert_eq!(result1, result2);
  }

  // Test map with empty HashMap
  #[test]
  fn test_map_empty() {
    let map: HashMap<String, i32> = HashMap::new();
    let result = map.map(|x| x * 2);

    assert!(result.is_empty());
  }

  // Test map with string conversion
  #[test]
  fn test_map_to_different_type() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let result = map.map(|x| x.to_string());

    let mut expected = HashMap::new();
    expected.insert("a", "1".to_string());
    expected.insert("b", "2".to_string());
    expected.insert("c", "3".to_string());

    assert_eq!(result, expected);
  }

  // Test map preserves key structure
  #[test]
  fn test_map_preserves_keys() {
    let mut map = HashMap::new();
    map.insert("key1", 10);
    map.insert("key2", 20);
    map.insert("key3", 30);

    let result = map.map(|x| x / 10);

    assert!(result.contains_key("key1"));
    assert!(result.contains_key("key2"));
    assert!(result.contains_key("key3"));
    assert_eq!(result["key1"], 1);
    assert_eq!(result["key2"], 2);
    assert_eq!(result["key3"], 3);
  }

  // Test with complex value types
  #[test]
  fn test_map_with_complex_values() {
    #[derive(Debug, Clone, PartialEq)]
    struct Person {
      name: String,
      age: i32,
    }

    let mut map = HashMap::new();
    map.insert(
      1,
      Person {
        name: "Alice".to_string(),
        age: 30,
      },
    );
    map.insert(
      2,
      Person {
        name: "Bob".to_string(),
        age: 40,
      },
    );

    let result = map.map(|p| p.age);

    let mut expected = HashMap::new();
    expected.insert(1, 30);
    expected.insert(2, 40);

    assert_eq!(result, expected);
  }

  // Test with different key types
  #[test]
  fn test_map_with_different_key_types() {
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct Key {
      id: u32,
      prefix: String,
    }

    let mut map = HashMap::new();
    map.insert(
      Key {
        id: 1,
        prefix: "A".to_string(),
      },
      100,
    );
    map.insert(
      Key {
        id: 2,
        prefix: "B".to_string(),
      },
      200,
    );

    let result = map.map(|v| v + 50);

    let key1 = Key {
      id: 1,
      prefix: "A".to_string(),
    };
    let key2 = Key {
      id: 2,
      prefix: "B".to_string(),
    };

    assert_eq!(result[&key1], 150);
    assert_eq!(result[&key2], 250);
  }

  // Test with stateful mapping function
  #[test]
  fn test_map_with_stateful_function() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    // Using a Vec to track mapped values - safer than mutable counter
    let values = [1, 2, 3];
    let mut used = [false, false, false];

    let result = map.map(move |x| {
      let idx = values.iter().position(|v| v == x).unwrap();
      used[idx] = true;
      *x * 10
    });

    // Check that result has the expected size
    assert_eq!(result.len(), 3);

    // Check that the mapped values are correct
    assert!(result.values().collect::<Vec<_>>().contains(&&10));
    assert!(result.values().collect::<Vec<_>>().contains(&&20));
    assert!(result.values().collect::<Vec<_>>().contains(&&30));
  }

  // Test that map with identity function preserves size
  #[test]
  fn test_map_preserves_size() {
    let mut map = HashMap::new();
    for i in 0..100 {
      map.insert(i.to_string(), i);
    }

    let result = map.map(|x| *x);
    assert_eq!(result.len(), 100);
  }

  // Property: for all HashMaps, after applying map with identity, we get back equivalent HashMap
  #[test]
  fn test_identity_prop() {
    // Different sizes of maps to test
    for size in &[0, 1, 5, 10, 50] {
      let mut map = HashMap::new();
      for i in 0..*size {
        map.insert(i.to_string(), i);
      }

      let result = map.clone().map(|x| *x);
      assert_eq!(result, map);
    }
  }

  // Property: for all HashMaps and all functions f and g, map(f).map(g) == map(x => g(f(x)))
  #[test]
  fn test_composition_prop() {
    // Different sizes of maps to test
    for size in &[0, 1, 5, 10, 50] {
      let mut map = HashMap::new();
      for i in 0..*size {
        map.insert(i.to_string(), i);
      }

      let f = |x: &i32| x * 2;
      let g = |x: &i32| x + 1;

      let result1 = map.clone().map(f).map(g);
      let result2 = map.clone().map(move |x| g(&f(x)));

      assert_eq!(result1, result2);
    }
  }
}
