use super::category::{BTreeMapCategory, BTreeMapFn};
use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BTreeMap;

impl<K: CloneableThreadSafe + Ord, V: CloneableThreadSafe> Arrow<V, V> for BTreeMapCategory<K, V> {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    BTreeMapFn::new(move |map| {
      map
        .into_iter()
        .map(|(k, v)| (k, f(v)))
        .collect::<BTreeMap<K, D>>()
    })
  }

  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    F: CloneableThreadSafe,
  >(
    f: Self::Morphism<V, V>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(V, C), (V, D)> {
    BTreeMapFn::new(move |map: BTreeMap<K, (V, C)>| {
      // Split the map into components
      let mut v_map = BTreeMap::new();
      let mut c_map = BTreeMap::new();

      for (k, (v, c)) in map {
        v_map.insert(k.clone(), v);
        c_map.insert(k, c);
      }

      // Apply f to the first component
      let v_result = f.apply(v_map);

      // Apply g to the second component
      let d_result = g.apply(c_map);

      // Reconstruct the result map
      let mut result = BTreeMap::new();
      for (k, v) in v_result {
        if let Some(d) = d_result.get(&k) {
          result.insert(k, (v, d.clone()));
        }
      }

      result
    })
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<V, V>,
    g: Self::Morphism<V, C>,
  ) -> Self::Morphism<V, (V, C)> {
    BTreeMapFn::new(move |map: BTreeMap<K, V>| {
      // Apply f to get V values
      let v_result = f.apply(map.clone());

      // Apply g to get C values
      let c_result = g.apply(map);

      // Combine the results
      let mut result = BTreeMap::new();
      for (k, v) in v_result {
        if let Some(c) = c_result.get(&k) {
          result.insert(k, (v, c.clone()));
        }
      }

      result
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::arrow::Arrow;
  use crate::traits::category::Category;
  use proptest::prelude::*;

  #[test]
  fn test_arrow_creation() {
    let f = BTreeMapCategory::<String, i32>::arrow(|x: i32| x * 2);
    let mut input = BTreeMap::new();
    input.insert("a".to_string(), 1);
    input.insert("b".to_string(), 2);

    let result = f.apply(input);
    assert_eq!(result.get("a"), Some(&2));
    assert_eq!(result.get("b"), Some(&4));
  }

  #[test]
  fn test_arrow_laws() {
    // Test that arrow() creates the same result as arr() for compatible functions
    let mut input = BTreeMap::new();
    input.insert("a".to_string(), 1);
    input.insert("b".to_string(), 2);

    let f_arrow = BTreeMapCategory::<String, i32>::arrow(|x: i32| x * 2);
    let f_arr = BTreeMapCategory::<String, i32>::arr(|x: &i32| x * 2);

    let result_arrow = f_arrow.apply(input.clone());
    let result_arr = f_arr.apply(input);

    assert_eq!(result_arrow, result_arr);
  }

  #[test]
  fn test_split() {
    let f = BTreeMapCategory::<String, i32>::arrow(|x: i32| x * 2);
    let g = BTreeMapCategory::<String, i32>::arrow(|x: i32| x + 1);

    let split_fn = BTreeMapCategory::<String, i32>::split::<i32, i32, i32, i32>(f, g);
    let mut input = BTreeMap::new();
    input.insert("a".to_string(), (1, 10));
    input.insert("b".to_string(), (2, 20));

    let result = split_fn.apply(input);
    assert_eq!(result.get("a"), Some(&(2, 11)));
    assert_eq!(result.get("b"), Some(&(4, 21)));
  }

  #[test]
  fn test_fanout() {
    let f = BTreeMapCategory::<String, i32>::arrow(|x: i32| x * 2);
    let g = BTreeMapCategory::<String, i32>::arrow(|x: i32| x + 1);

    let fanout_fn = BTreeMapCategory::<String, i32>::fanout(f, g);
    let mut input = BTreeMap::new();
    input.insert("a".to_string(), 1);
    input.insert("b".to_string(), 2);

    let result = fanout_fn.apply(input);
    assert_eq!(result.get("a"), Some(&(2, 2)));
    assert_eq!(result.get("b"), Some(&(4, 3)));
  }

  proptest! {
    #[test]
    fn prop_arrow_split_preserves_structure(
      xs in prop::collection::vec(any::<(String, i32)>(), 0..5)
    ) {
      let f = BTreeMapCategory::<String, i32>::arrow(|x: i32| x.saturating_mul(2));
      let g = BTreeMapCategory::<String, i32>::arrow(|x: i32| x.saturating_add(1));

      let split_fn = BTreeMapCategory::<String, i32>::split::<i32, i32, i32, i32>(f, g);
      let input: BTreeMap<String, (i32, i32)> = xs.iter()
        .map(|(k, v)| (k.clone(), (*v, v.saturating_mul(10))))
        .collect();

      let result = split_fn.apply(input.clone());

      // Check that the structure is preserved
      assert_eq!(result.len(), input.len());

      // Check that the transformation is correct
      for (k, (v, c)) in input.iter() {
        if let Some((expected_v, expected_c)) = result.get(k) {
          assert_eq!(*expected_v, v.saturating_mul(2));
          assert_eq!(*expected_c, c.saturating_add(1));
        }
      }
    }

    #[test]
    fn prop_arrow_fanout_preserves_structure(
      xs in prop::collection::vec(any::<(String, i32)>(), 0..5)
    ) {
      let f = BTreeMapCategory::<String, i32>::arrow(|x: i32| x.saturating_mul(2));
      let g = BTreeMapCategory::<String, i32>::arrow(|x: i32| x.saturating_add(1));

      let fanout_fn = BTreeMapCategory::<String, i32>::fanout(f, g);
      let input: BTreeMap<String, i32> = xs.iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();

      let result = fanout_fn.apply(input.clone());

      // Check that the structure is preserved
      assert_eq!(result.len(), input.len());

      // Check that the transformation is correct
      for (k, v) in input.iter() {
        if let Some((expected_v, expected_c)) = result.get(k) {
          assert_eq!(*expected_v, v.saturating_mul(2));
          assert_eq!(*expected_c, v.saturating_add(1));
        }
      }
    }
  }
}
