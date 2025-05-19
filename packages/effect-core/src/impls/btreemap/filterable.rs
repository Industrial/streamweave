use crate::impls::btreemap::category::BTreeMapCategory;
use crate::traits::filterable::Filterable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BTreeMap;
use std::marker::PhantomData;

// Implement Filterable for BTreeMapCategory
impl<K, A> Filterable<A> for BTreeMapCategory<K, A>
where
  K: Ord + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  type Filtered<B: CloneableThreadSafe> = BTreeMapCategory<K, B>;

  fn filter_map<B, F>(self, _f: F) -> Self::Filtered<B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // This is a placeholder implementation since BTreeMapCategory is just a proxy type
    // The actual filtering happens via the extension trait
    BTreeMapCategory(PhantomData)
  }
}

// Extension trait to make BTreeMap filtering more ergonomic
pub trait BTreeMapFilterableExt<K, A>
where
  K: Ord + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  fn filter_map<B, F>(self, f: F) -> BTreeMap<K, B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe;

  fn filter<F>(self, predicate: F) -> BTreeMap<K, A>
  where
    F: for<'a> FnMut(&'a A) -> bool + CloneableThreadSafe,
    A: Clone;
}

// Implement the extension trait for BTreeMap
impl<K, A> BTreeMapFilterableExt<K, A> for BTreeMap<K, A>
where
  K: Ord + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  fn filter_map<B, F>(self, mut f: F) -> BTreeMap<K, B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = BTreeMap::new();
    for (k, a) in self {
      if let Some(b) = f(&a) {
        result.insert(k.clone(), b);
      }
    }
    result
  }

  fn filter<F>(self, mut predicate: F) -> BTreeMap<K, A>
  where
    F: for<'a> FnMut(&'a A) -> bool + CloneableThreadSafe,
    A: Clone,
  {
    let mut result = BTreeMap::new();
    for (k, a) in self {
      if predicate(&a) {
        result.insert(k.clone(), a);
      }
    }
    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_filter_map_empty() {
    let map: BTreeMap<&str, i32> = BTreeMap::new();
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };

    let result = map.filter_map(f);
    assert_eq!(result, BTreeMap::<&str, String>::new());
  }

  #[test]
  fn test_filter_map_all_some() {
    let mut map = BTreeMap::new();
    map.insert("a", 20);
    map.insert("b", 30);
    map.insert("c", 40);

    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };

    let result = map.filter_map(f);

    let mut expected = BTreeMap::new();
    expected.insert("a", "20".to_string());
    expected.insert("b", "30".to_string());
    expected.insert("c", "40".to_string());

    assert_eq!(result, expected);
  }

  #[test]
  fn test_filter_map_some_none() {
    let mut map = BTreeMap::new();
    map.insert("a", 5);
    map.insert("b", 15);
    map.insert("c", 25);

    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };

    let result = map.filter_map(f);

    let mut expected = BTreeMap::new();
    expected.insert("b", "15".to_string());
    expected.insert("c", "25".to_string());

    assert_eq!(result, expected);
  }

  #[test]
  fn test_filter_map_all_none() {
    let mut map = BTreeMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };

    let result = map.filter_map(f);
    assert_eq!(result, BTreeMap::<&str, String>::new());
  }

  #[test]
  fn test_filter_empty() {
    let map: BTreeMap<&str, i32> = BTreeMap::new();
    let predicate = |x: &i32| *x > 10;

    let result = map.filter(predicate);
    assert_eq!(result, BTreeMap::<&str, i32>::new());
  }

  #[test]
  fn test_filter_all_pass() {
    let mut map = BTreeMap::new();
    map.insert("a", 20);
    map.insert("b", 30);
    map.insert("c", 40);

    let predicate = |x: &i32| *x > 10;

    let result = map.clone().filter(predicate);
    assert_eq!(result, map);
  }

  #[test]
  fn test_filter_some_pass() {
    let mut map = BTreeMap::new();
    map.insert("a", 5);
    map.insert("b", 15);
    map.insert("c", 25);

    let predicate = |x: &i32| *x > 10;

    let result = map.filter(predicate);

    let mut expected = BTreeMap::new();
    expected.insert("b", 15);
    expected.insert("c", 25);

    assert_eq!(result, expected);
  }

  #[test]
  fn test_filter_none_pass() {
    let mut map = BTreeMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let predicate = |x: &i32| *x > 10;

    let result = map.filter(predicate);
    assert_eq!(result, BTreeMap::<&str, i32>::new());
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_law(xs in prop::collection::vec(any::<(String, i32)>(), 0..10)) {
      // Identity law: filter_map(Some) == self
      let map: BTreeMap<_, _> = xs.clone().into_iter().collect();
      let identity = |val: &i32| Some(*val);

      let result = map.clone().filter_map(identity);

      // Check map equivalence
      for (k, v) in map {
        prop_assert_eq!(result.get(&k), Some(&v));
      }
      for (k, v) in result.clone() {
        prop_assert!(xs.iter().any(|(key, val)| key == &k && val == &v));
      }
    }

    #[test]
    fn prop_annihilation_law(xs in prop::collection::vec(any::<(String, i32)>(), 0..10)) {
      // Annihilation law: filter_map(|_| None) == empty
      let map: BTreeMap<_, _> = xs.into_iter().collect();
      let none_fn = |_: &i32| None::<i32>;

      let result = map.filter_map(none_fn);
      prop_assert_eq!(result, BTreeMap::<String, i32>::new());
    }

    #[test]
    fn prop_distributivity_law(xs in prop::collection::vec(any::<(String, i32)>(), 0..10), limit in 1..100i32) {
      // Distributivity law: filter_map(f).filter_map(g) == filter_map(|x| f(x).and_then(g))
      let map: BTreeMap<_, _> = xs.into_iter().collect();

      // Define filter functions
      let f = |val: &i32| if val.abs() % 2 == 0 { Some(*val) } else { None };
      let g = move |val: &i32| if val.abs() < limit { Some(val.to_string()) } else { None };

      // Apply filters sequentially
      let result1 = map.clone().filter_map(f).filter_map(g);

      // Apply composed filter
      let result2 = map.filter_map(move |val| {
        f(val).and_then(|v| g(&v))
      });

      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_filter_consistent_with_filter_map(xs in prop::collection::vec(any::<(String, i32)>(), 0..10), threshold in 1..100i32) {
      // filter(p) == filter_map(|x| if p(x) { Some(x) } else { None })
      let map: BTreeMap<_, _> = xs.into_iter().collect();

      let predicate = move |val: &i32| val.abs() < threshold;

      let result1 = map.clone().filter(predicate);
      let result2 = map.filter_map(move |val| {
        if predicate(val) { Some(*val) } else { None }
      });

      prop_assert_eq!(result1, result2);
    }
  }
}
