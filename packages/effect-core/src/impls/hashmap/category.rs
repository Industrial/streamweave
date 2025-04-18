use crate::{traits::category::Category, types::threadsafe::CloneableThreadSafe};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

// A cloneable function wrapper for HashMap
#[derive(Clone)]
pub struct HashMapFn<K, A, B>(Arc<dyn Fn(HashMap<K, A>) -> HashMap<K, B> + Send + Sync + 'static>);

impl<K, A, B> HashMapFn<K, A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(HashMap<K, A>) -> HashMap<K, B> + Send + Sync + 'static,
  {
    HashMapFn(Arc::new(f))
  }

  pub fn apply(&self, a: HashMap<K, A>) -> HashMap<K, B> {
    (self.0)(a)
  }
}

impl<K: CloneableThreadSafe + Eq + Hash, V: CloneableThreadSafe> Category<V, V> for HashMap<K, V> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = HashMapFn<K, A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    HashMapFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    HashMapFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(&A) -> B + CloneableThreadSafe,
  {
    HashMapFn::new(move |map: HashMap<K, A>| {
      map
        .iter()
        .map(|(k, a)| (k.clone(), f(a)))
        .collect::<HashMap<K, B>>()
    })
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    HashMapFn::new(move |map: HashMap<K, (A, C)>| {
      // Split the map into components
      let mut a_map = HashMap::new();
      let mut c_map = HashMap::new();

      for (k, (a, c)) in map {
        a_map.insert(k.clone(), a);
        c_map.insert(k, c);
      }

      // Apply f to the first component
      let b_map = f.apply(a_map);

      // Reconstruct the result map
      let mut result = HashMap::new();
      for (k, b) in b_map {
        if let Some(c) = c_map.get(&k) {
          result.insert(k, (b, c.clone()));
        }
      }

      result
    })
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    HashMapFn::new(move |map: HashMap<K, (C, A)>| {
      // Split the map into components
      let mut c_map = HashMap::new();
      let mut a_map = HashMap::new();

      for (k, (c, a)) in map {
        c_map.insert(k.clone(), c);
        a_map.insert(k, a);
      }

      // Apply f to the second component
      let b_map = f.apply(a_map);

      // Reconstruct the result map
      let mut result = HashMap::new();
      for (k, b) in b_map {
        if let Some(c) = c_map.get(&k) {
          result.insert(k, (c.clone(), b));
        }
      }

      result
    })
  }
}

#[cfg(test)]
mod tests {
  use crate::traits::category::Category;
  use proptest::prelude::*;
  use std::collections::HashMap;

  // Define test functions that operate on integers - using i64 instead of i32
  // and using checked arithmetic to prevent overflow
  const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  // Test the identity law: id() . f = f = f . id()
  proptest! {
      #[test]
      fn test_identity_law(
          // Use bounded input with smaller map size to prevent overflow
          kvs in proptest::collection::hash_map(
            any::<String>(),
            any::<i64>().prop_filter("Value too large", |v| *v < 10000),
            0..5
          ),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create our Category::arr version of the function
          let arr_f = <HashMap<String, i64> as Category<i64, i64>>::arr(f);

          // Get the identity morphism
          let id = <HashMap<String, i64> as Category<i64, i64>>::id();

          // Compose id . f
          let id_then_f = <HashMap<String, i64> as Category<i64, i64>>::compose(id.clone(), arr_f.clone());

          // Compose f . id
          let f_then_id = <HashMap<String, i64> as Category<i64, i64>>::compose(arr_f.clone(), id);

          // Apply the input to each composition
          let result_f = arr_f.apply(kvs.clone());
          let result_id_then_f = id_then_f.apply(kvs.clone());
          let result_f_then_id = f_then_id.apply(kvs);

          // All should give the same result
          assert_eq!(result_f, result_id_then_f);
          assert_eq!(result_f, result_f_then_id);
      }
  }

  // Test the composition law: (f . g) . h = f . (g . h)
  proptest! {
      #[test]
      fn test_composition_law(
          // Use bounded input with smaller map size to prevent overflow
          kvs in proptest::collection::hash_map(
            any::<String>(),
            any::<i64>().prop_filter("Value too large", |v| *v < 10000),
            0..5
          ),
          f_idx in 0..INT_FUNCTIONS.len(),
          g_idx in 0..INT_FUNCTIONS.len(),
          h_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get functions from our array
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <HashMap<String, i64> as Category<i64, i64>>::arr(f);
          let arr_g = <HashMap<String, i64> as Category<i64, i64>>::arr(g);
          let arr_h = <HashMap<String, i64> as Category<i64, i64>>::arr(h);

          // Compose (f . g) . h
          let fg = <HashMap<String, i64> as Category<i64, i64>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <HashMap<String, i64> as Category<i64, i64>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <HashMap<String, i64> as Category<i64, i64>>::compose(arr_g, arr_h);
          let f_gh = <HashMap<String, i64> as Category<i64, i64>>::compose(arr_f, gh);

          // Apply the input
          let result_fg_h = fg_h.apply(kvs.clone());
          let result_f_gh = f_gh.apply(kvs);

          // Both compositions should give the same result
          assert_eq!(result_fg_h, result_f_gh);
      }
  }

  // Test the first combinator - specific to HashMap implementation (key-value pairs)
  proptest! {
      #[test]
      fn test_first_combinator(
          // Use bounded input with smaller map size to prevent overflow
          kvs in proptest::collection::hash_map(
            any::<String>(),
            any::<i64>().prop_filter("Value too large", |v| *v < 10000),
            0..5
          ),
          cs in proptest::collection::vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <HashMap<String, i64> as Category<i64, i64>>::arr(f);

          // Apply first to get a function on pairs
          let first_f = <HashMap<String, i64> as Category<i64, i64>>::first(arr_f);

          // Create map of pairs as input
          let mut pairs_map = HashMap::new();
          for (key, value) in kvs.iter() {
              // Use the first element of cs as the second element of the pair
              // or a default if cs is empty
              let c = cs.first().copied().unwrap_or_default();
              pairs_map.insert(key.clone(), (*value, c));
          }

          // Apply the first combinator
          let result = first_f.apply(pairs_map.clone());

          // Compute the expected result
          let mut expected = HashMap::new();
          for (key, (a, c)) in pairs_map {
              expected.insert(key, (f(&a), c));
          }

          // Results should match
          assert_eq!(result, expected);
      }
  }

  // Test the second combinator - specific to HashMap implementation (key-value pairs)
  proptest! {
      #[test]
      fn test_second_combinator(
          // Use bounded input with smaller map size to prevent overflow
          kvs in proptest::collection::hash_map(
            any::<String>(),
            any::<i64>().prop_filter("Value too large", |v| *v < 10000),
            0..5
          ),
          cs in proptest::collection::vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <HashMap<String, i64> as Category<i64, i64>>::arr(f);

          // Apply second to get a function on pairs
          let second_f = <HashMap<String, i64> as Category<i64, i64>>::second(arr_f);

          // Create map of pairs as input
          let mut pairs_map = HashMap::new();
          for (key, value) in kvs.iter() {
              // Use the first element of cs as the first element of the pair
              // or a default if cs is empty
              let c = cs.first().copied().unwrap_or_default();
              pairs_map.insert(key.clone(), (c, *value));
          }

          // Apply the second combinator
          let result = second_f.apply(pairs_map.clone());

          // Compute the expected result
          let mut expected = HashMap::new();
          for (key, (c, a)) in pairs_map {
              expected.insert(key, (c, f(&a)));
          }

          // Results should match
          assert_eq!(result, expected);
      }
  }
}
