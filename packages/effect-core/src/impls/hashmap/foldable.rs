// This file contains a commented out implementation of Foldable for HashMap
// It requires a Functor implementation first
// To be uncommented once the Functor implementation is added

use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;
use std::hash::Hash;

impl<K, V> Foldable<V> for HashMap<K, V>
where
  K: Eq + Hash + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'b> FnMut(A, &'b V) -> A + CloneableThreadSafe,
  {
    let mut acc = init;
    for (_, v) in self {
      acc = f(acc, &v);
    }
    acc
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'b> FnMut(&'b V, A) -> A + CloneableThreadSafe,
  {
    // Note: HashMap iteration order is not guaranteed, but we'll convert
    // to Vec for consistency with other fold_right implementations
    let vec: Vec<_> = self.into_iter().collect();
    let mut acc = init;
    for (_, v) in vec.into_iter().rev() {
      acc = f(&v, acc);
    }
    acc
  }

  fn reduce<F>(self, mut f: F) -> Option<V>
  where
    F: for<'b, 'c> FnMut(&'b V, &'c V) -> V + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    let mut iter = self.into_iter();
    let (_, first) = iter.next().unwrap();
    let mut acc = first;

    for (_, v) in iter {
      acc = f(&acc, &v);
    }

    Some(acc)
  }

  fn reduce_right<F>(self, mut f: F) -> Option<V>
  where
    F: for<'b, 'c> FnMut(&'b V, &'c V) -> V + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    // Convert to Vec for reverse iteration
    let vec: Vec<_> = self.into_iter().collect();
    let mut rev_iter = vec.into_iter().rev();

    let (_, first) = rev_iter.next().unwrap();
    let mut acc = first;

    for (_, v) in rev_iter {
      acc = f(&v, &acc);
    }

    Some(acc)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashMap;

  #[test]
  fn test_empty_map() {
    let map: HashMap<i32, String> = HashMap::new();
    let result = map.fold(0, |acc, _| acc + 1);
    assert_eq!(result, 0);

    let map: HashMap<i32, String> = HashMap::new();
    let result = map.reduce(|a, _| a.clone());
    assert_eq!(result, None);
  }

  #[test]
  fn test_single_element() {
    let mut map = HashMap::new();
    map.insert(1, "one".to_string());

    let result = map.clone().fold(0, |acc, _| acc + 1);
    assert_eq!(result, 1);

    let result = map.clone().reduce(|a, _| a.clone());
    assert_eq!(result, Some("one".to_string()));
  }

  #[test]
  fn test_multiple_elements() {
    let mut map = HashMap::new();
    map.insert(1, "one".to_string());
    map.insert(2, "two".to_string());
    map.insert(3, "three".to_string());

    // Count elements
    let result = map.clone().fold(0, |acc, _| acc + 1);
    assert_eq!(result, 3);

    // Concatenate strings
    let result = map.clone().fold(String::new(), |mut acc, v| {
      if !acc.is_empty() {
        acc.push_str(", ");
      }
      acc.push_str(v);
      acc
    });

    // Since HashMap order is not guaranteed, we need to check all permutations
    let permutations = vec![
      "one, two, three",
      "one, three, two",
      "two, one, three",
      "two, three, one",
      "three, one, two",
      "three, two, one",
    ];
    assert!(permutations.contains(&result.as_str()));
  }

  #[test]
  fn test_fold_with_custom_accumulator() {
    let mut map = HashMap::new();
    map.insert("apple", 5);
    map.insert("banana", 10);
    map.insert("cherry", 15);

    // Sum the values
    let result = map.clone().fold(0, |acc, v| acc + v);
    assert_eq!(result, 30);

    // Create a vector of values
    let result = map.clone().fold(Vec::new(), |mut acc, v| {
      acc.push(*v);
      acc.sort();
      acc
    });
    assert_eq!(result, vec![5, 10, 15]);
  }

  #[test]
  fn test_reduce_with_concatenation() {
    let mut map = HashMap::new();
    map.insert(1, "Hello".to_string());
    map.insert(2, "World".to_string());

    // Combine values
    let result = map.reduce(|v1, v2| format!("{} {}", v1, v2));

    match result {
      Some(value) => {
        assert!(
          value == "Hello World" || value == "World Hello",
          "Expected 'Hello World' or 'World Hello', got '{}'",
          value
        );
      }
      None => panic!("Expected Some, got None"),
    }
  }
}
