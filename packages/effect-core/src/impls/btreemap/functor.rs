use crate::impls::btreemap::category::BTreeMapCategory;
use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BTreeMap;
use std::marker::PhantomData;

// Implement Functor for BTreeMapCategory
impl<K, V> Functor<V> for BTreeMapCategory<K, V>
where
  K: Ord + Clone + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  type HigherSelf<U: CloneableThreadSafe> = BTreeMapCategory<K, U>;

  fn map<U, F>(self, _f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a V) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    // This is a placeholder implementation since BTreeMapCategory is just a proxy type
    // The actual mapping happens via the Category impl when using arr and apply
    BTreeMapCategory(PhantomData)
  }

  fn map_owned<U, F>(self, _f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(V) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
    Self: Sized,
  {
    // This is a placeholder implementation since BTreeMapCategory is just a proxy type
    // The actual mapping happens via the Category impl when using arr and apply
    BTreeMapCategory(PhantomData)
  }
}

// Extension trait to make BTreeMap mapping more ergonomic
pub trait BTreeMapFunctorExt<K, V>
where
  K: Ord + Clone + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn map<U, F>(self, f: F) -> BTreeMap<K, U>
  where
    F: for<'a> FnMut(&'a V) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe;
}

// Implement the extension trait for BTreeMap
impl<K, V> BTreeMapFunctorExt<K, V> for BTreeMap<K, V>
where
  K: Ord + Clone + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn map<U, F>(self, mut f: F) -> BTreeMap<K, U>
  where
    F: for<'a> FnMut(&'a V) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    self.into_iter().map(|(k, v)| (k, f(&v))).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Helper functions for tests
  fn add_one(x: &i32) -> i32 {
    x + 1
  }
  fn double(x: &i32) -> i32 {
    // Use checked_mul to avoid overflow in tests
    x.checked_mul(2).unwrap_or(i32::MAX)
  }

  #[test]
  fn test_functor_map_extension() {
    let mut map = BTreeMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let mapped = map.map(|x| x * 2);

    assert_eq!(mapped.get("a"), Some(&2));
    assert_eq!(mapped.get("b"), Some(&4));
    assert_eq!(mapped.get("c"), Some(&6));
  }

  #[test]
  fn test_functor_identity() {
    let mut map = BTreeMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let mapped = map.clone().map(|x| *x);

    assert_eq!(mapped.get("a"), Some(&1));
    assert_eq!(mapped.get("b"), Some(&2));
    assert_eq!(mapped.get("c"), Some(&3));
  }

  #[test]
  fn test_functor_composition() {
    let mut map = BTreeMap::new();
    map.insert("a", 1);
    map.insert("b", 2);

    let map_then_map = map.clone().map(add_one).map(double);
    let composed = map.map(|x| double(&add_one(x)));

    assert_eq!(map_then_map, composed);
  }

  proptest! {
      #[test]
      fn prop_functor_identity(xs: Vec<(String, i32)>) {
          let map: BTreeMap<_, _> = xs.into_iter().collect();
          let mapped = map.clone().map(|v| *v);

          for (k, v) in map.iter() {
              prop_assert_eq!(mapped.get(k), Some(v));
          }
      }

      #[test]
      fn prop_functor_composition(xs: Vec<(String, i32)>) {
          let map: BTreeMap<_, _> = xs.into_iter().collect();

          let map_then_map = map.clone().map(add_one).map(double);
          let composed = map.map(|v| double(&add_one(v)));

          prop_assert_eq!(map_then_map, composed);
      }
  }
}
