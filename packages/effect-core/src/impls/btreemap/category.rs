use crate::{traits::category::Category, types::threadsafe::CloneableThreadSafe};
use std::collections::BTreeMap;
use std::sync::Arc;

// A cloneable function wrapper for BTreeMap
#[derive(Clone)]
pub struct BTreeMapFn<K, A, B>(
  Arc<dyn Fn(BTreeMap<K, A>) -> BTreeMap<K, B> + Send + Sync + 'static>,
);

impl<K, A, B> BTreeMapFn<K, A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(BTreeMap<K, A>) -> BTreeMap<K, B> + Send + Sync + 'static,
  {
    BTreeMapFn(Arc::new(f))
  }

  pub fn apply(&self, a: BTreeMap<K, A>) -> BTreeMap<K, B> {
    (self.0)(a)
  }
}

// Create a dummy type just to represent the BTreeMap category
// This is similar to the approach used for IteratorCategory
pub struct BTreeMapCategory<K, V>(std::marker::PhantomData<(K, V)>);

// Implement a specialization of Category trait for the case where:
// 1. The keys are Ord (required for BTreeMap)
// 2. The values are CloneableThreadSafe
impl<K: Clone + Ord + CloneableThreadSafe, V: CloneableThreadSafe> Category<V, V>
  for BTreeMapCategory<K, V>
{
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = BTreeMapFn<K, A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    BTreeMapFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    BTreeMapFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    BTreeMapFn::new(move |map: BTreeMap<K, A>| {
      map
        .into_iter()
        .map(|(k, a)| (k, f(&a)))
        .collect::<BTreeMap<K, B>>()
    })
  }

  // Implementation that doesn't require any additional bounds on C
  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    BTreeMapFn::new(move |map: BTreeMap<K, (A, C)>| {
      // Extract all entries from the input map
      let entries: Vec<(K, (A, C))> = map.into_iter().collect();

      // Group entries by their second component (C) using vector operations
      let mut group_indices: Vec<(usize, usize)> = Vec::new();
      let mut cs: Vec<C> = Vec::new();

      for (i, (_, (_, c))) in entries.iter().enumerate() {
        // Find the group index for this C value
        let group_idx = cs.iter().position(|existing_c| {
          // We need to compare references since we can't rely on C being Eq
          std::ptr::eq(existing_c as *const C, c as *const C)
        });

        match group_idx {
          Some(idx) => group_indices.push((i, idx)),
          None => {
            let new_idx = cs.len();
            cs.push(c.clone());
            group_indices.push((i, new_idx));
          }
        }
      }

      // Apply f to each group
      let mut result = BTreeMap::new();
      (0..cs.len()).for_each(|group_idx| {
        // Collect entries for this group
        let mut a_map = BTreeMap::new();
        for (entry_idx, g_idx) in &group_indices {
          if *g_idx == group_idx {
            let (k, (a, _)) = &entries[*entry_idx];
            a_map.insert(k.clone(), a.clone());
          }
        }

        // Apply f to this group
        let b_map = f.apply(a_map);

        // Create result pairs
        let c = &cs[group_idx];
        for (k, b) in b_map {
          result.insert(k, (b, c.clone()));
        }
      });

      result
    })
  }

  // Similar approach for second
  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    BTreeMapFn::new(move |map: BTreeMap<K, (C, A)>| {
      // Extract all entries from the input map
      let entries: Vec<(K, (C, A))> = map.into_iter().collect();

      // Group entries by their first component (C) using vector operations
      let mut group_indices: Vec<(usize, usize)> = Vec::new();
      let mut cs: Vec<C> = Vec::new();

      for (i, (_, (c, _))) in entries.iter().enumerate() {
        // Find the group index for this C value
        let group_idx = cs.iter().position(|existing_c| {
          // We need to compare references since we can't rely on C being Eq
          std::ptr::eq(existing_c as *const C, c as *const C)
        });

        match group_idx {
          Some(idx) => group_indices.push((i, idx)),
          None => {
            let new_idx = cs.len();
            cs.push(c.clone());
            group_indices.push((i, new_idx));
          }
        }
      }

      // Apply f to each group
      let mut result = BTreeMap::new();
      (0..cs.len()).for_each(|group_idx| {
        // Collect entries for this group
        let mut a_map = BTreeMap::new();
        for (entry_idx, g_idx) in &group_indices {
          if *g_idx == group_idx {
            let (k, (_, a)) = &entries[*entry_idx];
            a_map.insert(k.clone(), a.clone());
          }
        }

        // Apply f to this group
        let b_map = f.apply(a_map);

        // Create result pairs
        let c = &cs[group_idx];
        for (k, b) in b_map {
          result.insert(k, (c.clone(), b));
        }
      });

      result
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::collections::BTreeMap;

  // Helper function to create a BTreeMap from key-value pairs
  fn btreemap_from_pairs<K, V>(pairs: Vec<(K, V)>) -> BTreeMap<K, V>
  where
    K: Ord,
  {
    pairs.into_iter().collect()
  }

  // Test the identity law: id() . f = f = f . id()
  #[test]
  fn test_identity_law() {
    // Create a test map
    let map = btreemap_from_pairs(vec![
      (1, "one".to_string()),
      (2, "two".to_string()),
      (3, "three".to_string()),
    ]);

    // Define a simple function
    let f = BTreeMapCategory::<i32, String>::arr(|s: &String| s.len());

    // id() . f
    let id_then_f =
      BTreeMapCategory::<i32, String>::compose(BTreeMapCategory::<i32, String>::id(), f.clone());
    // f . id()
    let f_then_id =
      BTreeMapCategory::<i32, String>::compose(f.clone(), BTreeMapCategory::<i32, String>::id());

    // Apply all three functions to the map
    let result_f = f.apply(map.clone());
    let result_id_then_f = id_then_f.apply(map.clone());
    let result_f_then_id = f_then_id.apply(map.clone());

    // Results should be the same
    assert_eq!(result_f, result_id_then_f);
    assert_eq!(result_f, result_f_then_id);
  }

  // Test the composition law: (f . g) . h = f . (g . h)
  #[test]
  fn test_composition_law() {
    // Create a test map
    let map = btreemap_from_pairs(vec![(1, 5), (2, 10), (3, 15)]);

    // Define simple functions
    let f = BTreeMapCategory::<i32, i32>::arr(|x: &i32| x + 1);
    let g = BTreeMapCategory::<i32, i32>::arr(|x: &i32| x * 2);
    let h = BTreeMapCategory::<i32, i32>::arr(|x: &i32| x - 3);

    // (f . g) . h
    let fg = BTreeMapCategory::<i32, i32>::compose(f.clone(), g.clone());
    let fg_h = BTreeMapCategory::<i32, i32>::compose(fg, h.clone());

    // f . (g . h)
    let gh = BTreeMapCategory::<i32, i32>::compose(g.clone(), h.clone());
    let f_gh = BTreeMapCategory::<i32, i32>::compose(f.clone(), gh);

    // Apply both compositions to the map
    let result_fg_h = fg_h.apply(map.clone());
    let result_f_gh = f_gh.apply(map.clone());

    // Results should be the same
    assert_eq!(result_fg_h, result_f_gh);
  }

  // Test the arr function
  #[test]
  fn test_arr() {
    // Create a test map
    let map = btreemap_from_pairs(vec![
      (1, "one".to_string()),
      (2, "two".to_string()),
      (3, "three".to_string()),
    ]);

    // Define a function using arr
    let f = BTreeMapCategory::<i32, String>::arr(|s: &String| s.len());

    // Apply the function
    let result = f.apply(map);

    // Check the result
    let expected = btreemap_from_pairs(vec![
      (1, 3), // "one" has length 3
      (2, 3), // "two" has length 3
      (3, 5), // "three" has length 5
    ]);

    assert_eq!(result, expected);
  }

  // Test the first function
  #[test]
  fn test_first() {
    // Create a test map with pairs
    let map = btreemap_from_pairs(vec![
      (1, ("one".to_string(), 1)),
      (2, ("two".to_string(), 2)),
      (3, ("three".to_string(), 3)),
    ]);

    // Define a function to apply to the first component
    let f = BTreeMapCategory::<i32, String>::arr(|s: &String| s.len());

    // Apply first to get a morphism for pairs
    let first_f = BTreeMapCategory::<i32, (String, i32)>::first(f);

    // Apply the morphism
    let result = first_f.apply(map);

    // Check the result
    let expected = btreemap_from_pairs(vec![
      (1, (3, 1)), // "one" has length 3
      (2, (3, 2)), // "two" has length 3
      (3, (5, 3)), // "three" has length 5
    ]);

    assert_eq!(result, expected);
  }

  // Test the second function
  #[test]
  fn test_second() {
    // Create a test map with pairs
    let map = btreemap_from_pairs(vec![
      (1, (1, "one".to_string())),
      (2, (2, "two".to_string())),
      (3, (3, "three".to_string())),
    ]);

    // Define a function to apply to the second component
    let f = BTreeMapCategory::<i32, String>::arr(|s: &String| s.len());

    // Apply second to get a morphism for pairs
    let second_f = BTreeMapCategory::<i32, (i32, String)>::second(f);

    // Apply the morphism
    let result = second_f.apply(map);

    // Check the result
    let expected = btreemap_from_pairs(vec![
      (1, (1, 3)), // "one" has length 3
      (2, (2, 3)), // "two" has length 3
      (3, (3, 5)), // "three" has length 5
    ]);

    assert_eq!(result, expected);
  }

  // Property-based tests for Category laws
  proptest! {
    // Property test for identity law
    #[test]
    fn prop_identity_law(pairs in proptest::collection::vec((0..100i32, ".*".prop_map(String::from)), 0..10)) {
      let map: BTreeMap<i32, String> = pairs.into_iter().collect();

      // Define a simple function
      let f = BTreeMapCategory::<i32, String>::arr(|s: &String| s.len());

      // id() . f
      let id_then_f = BTreeMapCategory::<i32, String>::compose(
        BTreeMapCategory::<i32, String>::id(),
        f.clone()
      );
      // f . id()
      let f_then_id = BTreeMapCategory::<i32, String>::compose(
        f.clone(),
        BTreeMapCategory::<i32, String>::id()
      );

      // Apply all three functions to the map
      let result_f = f.apply(map.clone());
      let result_id_then_f = id_then_f.apply(map.clone());
      let result_f_then_id = f_then_id.apply(map.clone());

      // Results should be the same - use clone to avoid move errors
      prop_assert_eq!(result_f.clone(), result_id_then_f);
      prop_assert_eq!(result_f, result_f_then_id);
    }

    // Property test for composition law
    #[test]
    fn prop_composition_law(pairs in proptest::collection::vec((0..100i32, 0..100i32), 0..10)) {
      let map: BTreeMap<i32, i32> = pairs.into_iter().collect();

      // Define simple functions that perform safe integer operations
      let f = BTreeMapCategory::<i32, i32>::arr(|x: &i32| x.saturating_add(1));
      let g = BTreeMapCategory::<i32, i32>::arr(|x: &i32| x.saturating_mul(2));
      let h = BTreeMapCategory::<i32, i32>::arr(|x: &i32| x.saturating_sub(3));

      // (f . g) . h
      let fg = BTreeMapCategory::<i32, i32>::compose(f.clone(), g.clone());
      let fg_h = BTreeMapCategory::<i32, i32>::compose(fg, h.clone());

      // f . (g . h)
      let gh = BTreeMapCategory::<i32, i32>::compose(g.clone(), h.clone());
      let f_gh = BTreeMapCategory::<i32, i32>::compose(f.clone(), gh);

      // Apply both compositions to the map
      let result_fg_h = fg_h.apply(map.clone());
      let result_f_gh = f_gh.apply(map.clone());

      // Results should be the same
      prop_assert_eq!(result_fg_h, result_f_gh);
    }

    // Property test for arr preserving map structure
    #[test]
    fn prop_arr_preserves_structure(
      // Use a unique set of keys to ensure the test is valid
      raw_pairs in proptest::collection::hash_set(any::<i32>(), 0..10).prop_map(|keys| {
        keys.into_iter().map(|k| (k, format!("{}", k))).collect::<Vec<_>>()
      })
    ) {
      // Convert raw_pairs to a map
      let map: BTreeMap<i32, String> = raw_pairs.clone().into_iter().collect();

      // Apply arr with a simple function
      let f = BTreeMapCategory::<i32, String>::arr(|s: &String| s.len());
      let result = f.apply(map);

      // Check that the keys are preserved
      for (k, _) in raw_pairs.iter() {
        prop_assert!(result.contains_key(k));
      }

      // Check that the values are transformed correctly
      for (k, v) in raw_pairs.iter() {
        // Store len in a variable to avoid temporary value error
        let len = v.len();
        prop_assert_eq!(result.get(k), Some(&len));
      }
    }
  }
}
