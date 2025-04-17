//! Groupable trait and implementations.
//!
//! A groupable is a type that can be grouped by a key function.

use std::collections::HashMap;
use std::hash::Hash;

/// A trait for types that can be grouped by a key function.
pub trait Groupable<K> {
  type HigherSelf;

  /// Groups elements by a key function and reduces values with the same key.
  ///
  /// # Arguments
  /// * `f` - A function that extracts a key from an element
  /// * `init` - A function that creates the initial value from an element
  /// * `g` - A function that combines two values with the same key
  fn group_by_reduce<R: Clone, F, I, G>(self, f: F, init: I, g: G) -> HashMap<K, R>
  where
    F: Fn(&Self::HigherSelf) -> K,
    I: Fn(&Self::HigherSelf) -> R,
    G: Fn(&R, &Self::HigherSelf) -> R,
    K: Eq + Hash;
}

// Implementation for Vec
impl<T: Send + Sync + Clone> Groupable<T> for Vec<T> {
  type HigherSelf = T;

  fn group_by_reduce<R: Clone, F, I, G>(self, f: F, init: I, g: G) -> HashMap<T, R>
  where
    F: Fn(&T) -> T,
    I: Fn(&T) -> R,
    G: Fn(&R, &T) -> R,
    T: Eq + Hash,
  {
    let mut groups: HashMap<T, R> = HashMap::new();

    for item in self {
      let key = f(&item);
      if let Some(existing) = groups.get(&key) {
        let new_value = g(existing, &item);
        groups.insert(key, new_value);
      } else {
        groups.insert(key, init(&item));
      }
    }

    groups
  }
}

// Implementation for HashMap
impl<K: Eq + Hash + Clone, V: Clone> Groupable<K> for HashMap<K, V> {
  type HigherSelf = V;

  fn group_by_reduce<R: Clone, F, I, G>(self, f: F, init: I, g: G) -> HashMap<K, R>
  where
    F: Fn(&V) -> K,
    I: Fn(&V) -> R,
    G: Fn(&R, &V) -> R,
  {
    let mut groups: HashMap<K, R> = HashMap::new();

    for (_, value) in self {
      let key = f(&value);
      if let Some(existing) = groups.get(&key) {
        let new_value = g(existing, &value);
        groups.insert(key, new_value);
      } else {
        groups.insert(key, init(&value));
      }
    }

    groups
  }
}

impl<T: Clone> Groupable<T> for Option<T> {
  type HigherSelf = T;

  fn group_by_reduce<R: Clone, F, I, G>(self, f: F, init: I, g: G) -> HashMap<T, R>
  where
    F: Fn(&T) -> T,
    I: Fn(&T) -> R,
    G: Fn(&R, &T) -> R,
    T: Eq + Hash,
  {
    let mut groups: HashMap<T, R> = HashMap::new();

    if let Some(value) = self {
      let key = f(&value);
      groups.insert(key, init(&value));
    }

    groups
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_vec_group_by_reduce() {
    let v = vec!["hello", "world"];
    let groups = v.group_by_reduce(
      |s: &&str| if (*s).len() == 5 { "five" } else { "other" },
      |s: &&str| (*s).to_string(),
      |acc: &String, s: &&str| format!("{}, {}", acc, s),
    );

    let mut expected = HashMap::new();
    expected.insert("five", "hello, world".to_string());
    assert_eq!(groups, expected);
  }

  #[test]
  fn test_vec_group_by_reduce_numbers() {
    let v = vec![1, 2, 3, 4, 5, 6];
    let groups = v.group_by_reduce(|x: &i32| x % 2, |x: &i32| *x, |acc: &i32, x: &i32| acc + x);

    let mut expected = HashMap::new();
    expected.insert(1, 9);
    expected.insert(0, 12);
    assert_eq!(groups, expected);
  }

  #[test]
  fn test_option_group_by_reduce() {
    let opt = Some(42);
    let groups = opt.group_by_reduce(|x: &i32| x % 2, |x: &i32| *x, |acc: &i32, x: &i32| acc + x);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups.get(&0).unwrap(), &42);
  }
}
