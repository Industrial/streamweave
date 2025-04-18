use crate::{traits::Category, threadsafe::CloneableThreadSafe};
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
  use super::*;
  use crate::traits::Category;
  use proptest::collection::hash_map;
  use proptest::prelude::*;
  use std::collections::HashMap;

  // Define test functions that operate on integers
  const INT_FUNCTIONS: &[fn(&i32) -> i32] = &[
    |x| x + 1,
    |x| x * 2,
    |x| x - 1,
    |x| x / 2,
    |x| x * x,
    |x| -x,
  ];

  // Test the identity law: id() . f = f = f . id()
  proptest! {
      #[test]
      fn test_identity_law(
          map in hash_map(any::<String>(), any::<i32>(), 0..10),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create our Category::arr version of the function
          let arr_f = <HashMap<String, i32> as Category<i32, i32>>::arr(f);

          // Get the identity morphism
          let id = <HashMap<String, i32> as Category<i32, i32>>::id();

          // Compose id . f
          let id_then_f = <HashMap<String, i32> as Category<i32, i32>>::compose(id.clone(), arr_f.clone());

          // Compose f . id
          let f_then_id = <HashMap<String, i32> as Category<i32, i32>>::compose(arr_f.clone(), id);

          // Apply the input to each composition
          let result_f = arr_f.apply(map.clone());
          let result_id_then_f = id_then_f.apply(map.clone());
          let result_f_then_id = f_then_id.apply(map);

          // All should give the same result
          assert_eq!(result_f, result_id_then_f);
          assert_eq!(result_f, result_f_then_id);
      }
  }

  // Test the composition law: (f . g) . h = f . (g . h)
  proptest! {
      #[test]
      fn test_composition_law(
          map in hash_map(any::<String>(), any::<i32>(), 0..10),
          f_idx in 0..INT_FUNCTIONS.len(),
          g_idx in 0..INT_FUNCTIONS.len(),
          h_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get functions from our array
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <HashMap<String, i32> as Category<i32, i32>>::arr(f);
          let arr_g = <HashMap<String, i32> as Category<i32, i32>>::arr(g);
          let arr_h = <HashMap<String, i32> as Category<i32, i32>>::arr(h);

          // Compose (f . g) . h
          let fg = <HashMap<String, i32> as Category<i32, i32>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <HashMap<String, i32> as Category<i32, i32>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <HashMap<String, i32> as Category<i32, i32>>::compose(arr_g, arr_h);
          let f_gh = <HashMap<String, i32> as Category<i32, i32>>::compose(arr_f, gh);

          // Apply the input
          let result_fg_h = fg_h.apply(map.clone());
          let result_f_gh = f_gh.apply(map);

          // Both compositions should give the same result
          assert_eq!(result_fg_h, result_f_gh);
      }
  }

  // Test the first combinator
  proptest! {
      #[test]
      fn test_first_combinator(
          keys in proptest::collection::vec(any::<String>(), 1..10),
          xs in proptest::collection::vec(any::<i32>(), 1..10),
          cs in proptest::collection::vec(any::<i32>(), 1..10),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Ensure all vectors have the same length
          let min_len = std::cmp::min(std::cmp::min(keys.len(), xs.len()), cs.len());
          let keys = keys.into_iter().take(min_len).collect::<Vec<_>>();
          let xs = xs.into_iter().take(min_len).collect::<Vec<_>>();
          let cs = cs.into_iter().take(min_len).collect::<Vec<_>>();

          // Create a map of pairs
          let mut pairs_map = HashMap::new();
          for i in 0..min_len {
              pairs_map.insert(keys[i].clone(), (xs[i], cs[i]));
          }

          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <HashMap<String, i32> as Category<i32, i32>>::arr(f);

          // Apply first to get a function on pairs
          let first_f = <HashMap<String, i32> as Category<i32, i32>>::first(arr_f);

          // Apply the first combinator
          let result = first_f.apply(pairs_map.clone());

          // Compute the expected result
          let mut expected = HashMap::new();
          for (k, (a, c)) in pairs_map {
              expected.insert(k, (f(&a), c));
          }

          // Results should match
          assert_eq!(result, expected);
      }
  }

  // Test the second combinator
  proptest! {
      #[test]
      fn test_second_combinator(
          keys in proptest::collection::vec(any::<String>(), 1..10),
          xs in proptest::collection::vec(any::<i32>(), 1..10),
          cs in proptest::collection::vec(any::<i32>(), 1..10),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Ensure all vectors have the same length
          let min_len = std::cmp::min(std::cmp::min(keys.len(), xs.len()), cs.len());
          let keys = keys.into_iter().take(min_len).collect::<Vec<_>>();
          let xs = xs.into_iter().take(min_len).collect::<Vec<_>>();
          let cs = cs.into_iter().take(min_len).collect::<Vec<_>>();

          // Create a map of pairs
          let mut pairs_map = HashMap::new();
          for i in 0..min_len {
              pairs_map.insert(keys[i].clone(), (cs[i], xs[i]));
          }

          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <HashMap<String, i32> as Category<i32, i32>>::arr(f);

          // Apply second to get a function on pairs
          let second_f = <HashMap<String, i32> as Category<i32, i32>>::second(arr_f);

          // Apply the second combinator
          let result = second_f.apply(pairs_map.clone());

          // Compute the expected result
          let mut expected = HashMap::new();
          for (k, (c, a)) in pairs_map {
              expected.insert(k, (c, f(&a)));
          }

          // Results should match
          assert_eq!(result, expected);
      }
  }
}
