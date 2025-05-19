use crate::traits::filterable::Filterable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;
use std::hash::Hash;

impl<K, A> Filterable<A> for HashMap<K, A>
where
  K: Eq + Hash + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  type Filtered<B: CloneableThreadSafe> = HashMap<K, B>;

  fn filter_map<B, F>(self, mut f: F) -> Self::Filtered<B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = HashMap::with_capacity(self.len());
    for (k, a) in self {
      if let Some(b) = f(&a) {
        result.insert(k.clone(), b);
      }
    }
    result
  }

  // We can override the default filter implementation for better performance
  fn filter<F>(self, mut predicate: F) -> Self::Filtered<A>
  where
    F: for<'a> FnMut(&'a A) -> bool + CloneableThreadSafe,
    A: Clone,
  {
    let mut result = HashMap::with_capacity(self.len());
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
    let map: HashMap<&str, i32> = HashMap::new();
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };

    let result = Filterable::filter_map(map, f);
    assert_eq!(result, HashMap::<&str, String>::new());
  }

  #[test]
  fn test_filter_map_all_some() {
    let mut map = HashMap::new();
    map.insert("a", 20);
    map.insert("b", 30);
    map.insert("c", 40);

    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };

    let result = Filterable::filter_map(map, f);

    let mut expected = HashMap::new();
    expected.insert("a", "20".to_string());
    expected.insert("b", "30".to_string());
    expected.insert("c", "40".to_string());

    assert_eq!(result, expected);
  }

  #[test]
  fn test_filter_map_some_none() {
    let mut map = HashMap::new();
    map.insert("a", 5);
    map.insert("b", 15);
    map.insert("c", 25);

    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };

    let result = Filterable::filter_map(map, f);

    let mut expected = HashMap::new();
    expected.insert("b", "15".to_string());
    expected.insert("c", "25".to_string());

    assert_eq!(result, expected);
  }

  #[test]
  fn test_filter_map_all_none() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };

    let result = Filterable::filter_map(map, f);
    assert_eq!(result, HashMap::<&str, String>::new());
  }

  #[test]
  fn test_filter_empty() {
    let map: HashMap<&str, i32> = HashMap::new();
    let predicate = |x: &i32| *x > 10;

    let result = Filterable::filter(map, predicate);
    assert_eq!(result, HashMap::<&str, i32>::new());
  }

  #[test]
  fn test_filter_all_pass() {
    let mut map = HashMap::new();
    map.insert("a", 20);
    map.insert("b", 30);
    map.insert("c", 40);

    let predicate = |x: &i32| *x > 10;

    let result = Filterable::filter(map.clone(), predicate);
    assert_eq!(result, map);
  }

  #[test]
  fn test_filter_some_pass() {
    let mut map = HashMap::new();
    map.insert("a", 5);
    map.insert("b", 15);
    map.insert("c", 25);

    let predicate = |x: &i32| *x > 10;

    let result = Filterable::filter(map, predicate);

    let mut expected = HashMap::new();
    expected.insert("b", 15);
    expected.insert("c", 25);

    assert_eq!(result, expected);
  }

  #[test]
  fn test_filter_none_pass() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let predicate = |x: &i32| *x > 10;

    let result = Filterable::filter(map, predicate);
    assert_eq!(result, HashMap::<&str, i32>::new());
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_law(keys in prop::collection::vec(".*", 0..10), values in prop::collection::vec(any::<i32>(), 0..10)) {
      // Identity law: filter_map(Some) == self
      let map: HashMap<String, i32> = keys.into_iter().zip(values.iter().cloned()).collect();
      let map_clone = map.clone();
      let identity = |val: &i32| Some(*val);

      let result = Filterable::filter_map(map, identity);
      prop_assert_eq!(result, map_clone);
    }

    #[test]
    fn prop_annihilation_law(keys in prop::collection::vec(".*", 0..10), values in prop::collection::vec(any::<i32>(), 0..10)) {
      // Annihilation law: filter_map(|_| None) == empty
      let map: HashMap<String, i32> = keys.into_iter().zip(values.iter().cloned()).collect();
      let none_fn = |_: &i32| None::<i32>;

      let result = Filterable::filter_map(map, none_fn);
      prop_assert_eq!(result, HashMap::<String, i32>::new());
    }

    #[test]
    fn prop_distributivity_law(keys in prop::collection::vec(".*", 0..10), values in prop::collection::vec(any::<i32>(), 0..10), limit in 1..100i32) {
      // Distributivity law: filter_map(f).filter_map(g) == filter_map(|x| f(x).and_then(g))
      let map: HashMap<String, i32> = keys.into_iter().zip(values.iter().cloned()).collect();

      // Define filter functions
      let f = |val: &i32| if val.abs() % 2 == 0 { Some(*val) } else { None };
      let g = move |val: &i32| if val.abs() < limit { Some(val.to_string()) } else { None };

      // Apply filters sequentially
      let result1 = Filterable::filter_map(map.clone(), f);
      let result1 = Filterable::filter_map(result1, g);

      // Apply composed filter
      let result2 = Filterable::filter_map(map, move |val| {
        f(val).and_then(|v| g(&v))
      });

      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_filter_consistent_with_filter_map(keys in prop::collection::vec(".*", 0..10), values in prop::collection::vec(any::<i32>(), 0..10), threshold in 1..100i32) {
      // filter(p) == filter_map(|x| if p(x) { Some(x) } else { None })
      let map: HashMap<String, i32> = keys.into_iter().zip(values.iter().cloned()).collect();

      let predicate = move |val: &i32| val.abs() < threshold;

      let result1 = Filterable::filter(map.clone(), predicate);
      let result2 = Filterable::filter_map(map, move |val| {
        if predicate(val) { Some(*val) } else { None }
      });

      prop_assert_eq!(result1, result2);
    }
  }
}
