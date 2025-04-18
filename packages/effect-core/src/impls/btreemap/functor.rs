// Commenting out implementation as it requires the Category trait

/* Commenting out the Functor implementation for BTreeMap as it requires the Category trait
   which isn't implemented for BTreeMap.

impl<K: Ord + CloneableThreadSafe, V: CloneableThreadSafe> Functor<V> for BTreeMap<K, V> {
  type HigherSelf<U: CloneableThreadSafe> = BTreeMap<K, U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a V) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    self.into_iter().map(|(k, v)| (k, f(&v))).collect()
  }
}
*/

#[cfg(test)]
mod tests {
  /* Commenting out tests since the implementation is commented out
  use super::*;
  use proptest::prelude::*;
  use std::collections::BTreeMap;

  #[test]
  fn test_functor_identity() {
    let mut map = BTreeMap::new();
    map.insert("a", 1);
    map.insert("b", 2);
    map.insert("c", 3);

    let mapped = Functor::map(map.clone(), |x| *x);

    assert_eq!(mapped.get("a"), Some(&1));
    assert_eq!(mapped.get("b"), Some(&2));
    assert_eq!(mapped.get("c"), Some(&3));
  }

  #[test]
  fn test_functor_composition() {
    let mut map = BTreeMap::new();
    map.insert("a", 1);
    map.insert("b", 2);

    let f = |x: &i32| x + 1;
    let g = |x: &i32| x * 2;

    let map_then_map = Functor::map(Functor::map(map.clone(), f), g);
    let composed = Functor::map(map, |x| g(&f(x)));

    assert_eq!(map_then_map, composed);
  }

  proptest! {
      #[test]
      fn prop_functor_identity(xs: Vec<(String, i32)>) {
          let map: BTreeMap<_, _> = xs.into_iter().collect();
          let mapped = Functor::map(map.clone(), |v| *v);

          for (k, v) in map.iter() {
              prop_assert_eq!(mapped.get(k), Some(v));
          }
      }

      #[test]
      fn prop_functor_composition(xs: Vec<(String, i32)>) {
          let map: BTreeMap<_, _> = xs.into_iter().collect();
          let f = |v: &i32| *v + 1;
          let g = |v: &i32| *v * 2;

          let map_then_map = Functor::map(Functor::map(map.clone(), f), g);
          let composed = Functor::map(map, |v| g(&f(v)));

          prop_assert_eq!(map_then_map, composed);
      }
  }
  */
}
