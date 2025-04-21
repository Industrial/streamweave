use crate::impls::btreeset::category::BTreeSetCategory;
use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BTreeSet;

// Implement Functor for BTreeSetCategory, not for BTreeSet directly
impl<T: CloneableThreadSafe> Functor<T> for BTreeSetCategory {
  type HigherSelf<U: CloneableThreadSafe> = BTreeSetCategory;

  fn map<U, F>(self, _f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    // This is a placeholder implementation since BTreeSetCategory is just a proxy type
    // The actual mapping happens via the Category impl when using arr and apply
    BTreeSetCategory
  }
}

// Extension trait to make BTreeSet mapping more ergonomic
pub trait BTreeSetFunctorExt<T: CloneableThreadSafe + Ord> {
  fn map<U, F>(self, f: F) -> BTreeSet<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe + Ord;
}

// Implement the extension trait for BTreeSet
impl<T: CloneableThreadSafe + Ord> BTreeSetFunctorExt<T> for BTreeSet<T> {
  fn map<U, F>(self, mut f: F) -> BTreeSet<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe + Ord,
  {
    self.into_iter().map(|x| f(&x)).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Helper function to create a BTreeSet from a Vec
  fn to_btreeset<T: Ord + Clone>(v: Vec<T>) -> BTreeSet<T> {
    BTreeSet::from_iter(v)
  }

  // Define test functions with overflow protection
  const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  #[test]
  fn test_identity_law_empty() {
    let set: BTreeSet<i64> = BTreeSet::new();
    let id = |x: &i64| *x;
    let result = set.map(id);

    // Identity law for empty set
    assert_eq!(result, BTreeSet::<i64>::new());
  }

  #[test]
  fn test_identity_law_nonempty() {
    let xs = to_btreeset(vec![1, 2, 3]);
    let set = xs.clone();
    let id = |x: &i64| *x;
    let result = set.map(id);

    // Identity law: functor.map(id) == functor
    assert_eq!(result, xs);
  }

  #[test]
  fn test_duplicate_elements() {
    // BTreeSet retains only unique values, so mapping might reduce the number of elements
    let set = to_btreeset(vec![1, 2, 3, 4]);
    let result = set.map(|x| x % 3); // Maps to [0, 1, 2, 1]

    // After inserting into a set, duplicates are removed, so we expect [0, 1, 2]
    let expected = to_btreeset(vec![0, 1, 2]);
    assert_eq!(result, expected);
  }

  proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn test_identity_law_prop(
      xs in prop::collection::vec(
        any::<i64>().prop_filter("Value too large", |v| *v < 10000),
        0..5
      )
    ) {
      let set: BTreeSet<i64> = xs.iter().cloned().collect();
      let id = |x: &i64| *x;
      let result = set.map(id);

      let expected: BTreeSet<i64> = xs.iter().cloned().collect();
      // Identity law: functor.map(id) == functor
      prop_assert_eq!(result, expected);
    }

    #[test]
    fn test_composition_law_empty(
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];
      let set: BTreeSet<i64> = BTreeSet::new();

      // Create move closures to avoid borrowing
      let f_captured = f;
      let g_captured = g;

      // Apply f then g
      let result1 = set.clone().map(f_captured).map(g_captured);

      // Apply composition of f and g
      let result2 = set.map(move |x| g_captured(&f_captured(x)));

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn test_composition_law_nonempty(
      xs in prop::collection::vec(
        any::<i64>().prop_filter("Value too large", |v| *v < 10000).prop_map(|x| x.abs() % 100),
        1..10
      ),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      // Use BTreeSet to remove any duplicates in the input
      let set: BTreeSet<i64> = xs.into_iter().collect();
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      // Create move closures to avoid borrowing
      let f_captured = f;
      let g_captured = g;

      // Apply f then g
      let result1 = set.clone().map(f_captured).map(g_captured);

      // Apply composition of f and g
      let result2 = set.clone().map(move |x| g_captured(&f_captured(x)));

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }

    // Implementation-specific test: element-wise operations
    #[test]
    fn test_element_wise_mapping(
      xs in prop::collection::vec(
        any::<i64>().prop_filter("Value too large", |v| *v < 10000),
        1..5
      ),
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      // First, create a BTreeSet to eliminate duplicates
      let set: BTreeSet<i64> = xs.iter().cloned().collect();
      let f = INT_FUNCTIONS[f_idx];
      let result = set.clone().map(f);

      // Since mapping might create duplicates, we need to manually compute
      // what the expected set should be
      let expected: BTreeSet<i64> = set.iter().map(f).collect();

      // Verify element-wise mapping
      prop_assert_eq!(result, expected);
    }
  }
}
