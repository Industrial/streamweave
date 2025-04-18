use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;
use std::hash::Hash;

// Implement Foldable for HashMap that aligns with the Functor implementation
// which has type parameter A, not (K, V)
impl<K: Eq + Hash + Clone + CloneableThreadSafe, V: CloneableThreadSafe> Foldable<V>
  for HashMap<K, V>
{
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a V) -> A + CloneableThreadSafe,
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
    F: for<'a> FnMut(&'a V, A) -> A + CloneableThreadSafe,
  {
    // For HashMap, fold_right doesn't have a natural order, so we collect into a Vec first
    let values: Vec<V> = self.into_iter().map(|(_, v)| v).collect();
    let mut acc = init;
    for v in values.into_iter().rev() {
      acc = f(&v, acc);
    }
    acc
  }

  fn reduce<F>(self, mut f: F) -> Option<V>
  where
    F: for<'a, 'b> FnMut(&'a V, &'b V) -> V + CloneableThreadSafe,
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
    F: for<'a, 'b> FnMut(&'a V, &'b V) -> V + CloneableThreadSafe,
  {
    if self.is_empty() {
      return None;
    }

    // For HashMap, reduce_right doesn't have a natural order, so we collect into a Vec first
    let values: Vec<V> = self.into_iter().map(|(_, v)| v).collect();
    let mut iter = values.iter().rev();

    let first = iter.next().unwrap();
    let mut acc = first.clone();

    for v in iter {
      acc = f(v, &acc);
    }

    Some(acc)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_fold() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let sum = Foldable::fold(map, 0, |acc, v| acc + v);
    assert_eq!(sum, 6); // 1 + 2 + 3 = 6
  }

  #[test]
  fn test_fold_right() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    // For fold_right, we don't know the exact order, but we can test properties
    let sum = Foldable::fold_right(map, 0, |v, acc| acc + v);
    assert_eq!(sum, 6); // 1 + 2 + 3 = 6 (in any order)
  }

  #[test]
  fn test_reduce() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let sum = Foldable::reduce(map, |a, b| a + b).unwrap();
    assert_eq!(sum, 6); // 1 + 2 + 3 = 6
  }

  #[test]
  fn test_reduce_right() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    // For reduce_right, we don't know the exact order, but we can test properties
    let sum = Foldable::reduce_right(map, |a, b| a + b).unwrap();
    assert_eq!(sum, 6); // 1 + 2 + 3 = 6 (in any order)
  }

  proptest! {
    #[test]
    fn prop_fold_sum(xs: Vec<(String, i8)>) {
      let map: HashMap<_, _> = xs.iter().cloned().collect();
      let map_clone = map.clone();

      // Use fold directly on the map entries to get the expected sum
      let expected_sum: i32 = map_clone.values().fold(0i32, |acc, &v| acc + v as i32);

      // Use Foldable::fold to get the actual sum
      let actual_sum = Foldable::fold(map, 0i32, |acc: i32, &v| acc + v as i32);

      prop_assert_eq!(actual_sum, expected_sum);
    }

    #[test]
    fn prop_fold_right_sum(xs: Vec<(String, i8)>) {
      let map: HashMap<_, _> = xs.iter().cloned().collect();
      let map_clone = map.clone();

      // Use fold directly on the map entries to get the expected sum
      let expected_sum: i32 = map_clone.values().fold(0i32, |acc, &v| acc + v as i32);

      // Use Foldable::fold_right to get the actual sum - addition is commutative
      // so order doesn't matter for the sum
      let actual_sum = Foldable::fold_right(map, 0i32, |&v, acc: i32| acc + v as i32);

      prop_assert_eq!(actual_sum, expected_sum);
    }
  }
}
