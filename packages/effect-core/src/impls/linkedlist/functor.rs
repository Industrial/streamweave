use crate::impls::linkedlist::category::LinkedListCategory;
use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::LinkedList;

// Implement Functor for LinkedListCategory, not for LinkedList directly
impl<T: CloneableThreadSafe> Functor<T> for LinkedListCategory {
  type HigherSelf<U: CloneableThreadSafe> = LinkedListCategory;

  fn map<U, F>(self, _f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    // This is a placeholder implementation since LinkedListCategory is just a proxy type
    // The actual mapping happens via the Category impl when using arr and apply
    LinkedListCategory
  }
}

// Extension trait to make LinkedList mapping more ergonomic
pub trait LinkedListFunctorExt<T: CloneableThreadSafe> {
  fn map<U, F>(self, f: F) -> LinkedList<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe;
}

// Implement the extension trait for LinkedList
impl<T: CloneableThreadSafe> LinkedListFunctorExt<T> for LinkedList<T> {
  fn map<U, F>(self, mut f: F) -> LinkedList<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    self.into_iter().map(|x| f(&x)).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

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
    let list: LinkedList<i64> = LinkedList::new();
    let id = |x: &i64| *x;
    let result = list.map(id);

    // Identity law for empty list
    assert_eq!(result, LinkedList::<i64>::new());
  }

  #[test]
  fn test_identity_law_nonempty() {
    let mut xs = LinkedList::new();
    xs.push_back(1);
    xs.push_back(2);
    xs.push_back(3);
    let list = xs.clone();
    let id = |x: &i64| *x;
    let result = list.map(id);

    // Identity law: functor.map(id) == functor
    assert_eq!(result, xs);
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
      let list: LinkedList<i64> = xs.iter().cloned().collect();
      let id = |x: &i64| *x;
      let result = list.map(id);

      let expected: LinkedList<i64> = xs.iter().cloned().collect();
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
      let list: LinkedList<i64> = LinkedList::new();

      // Create move closures to avoid borrowing
      let f_captured = f;
      let g_captured = g;

      // Apply f then g
      let result1 = list.clone().map(f_captured).map(g_captured);

      // Apply composition of f and g
      let result2 = list.map(move |x| g_captured(&f_captured(x)));

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn test_composition_law_nonempty(
      xs in prop::collection::vec(
        any::<i64>().prop_filter("Value too large", |v| *v < 10000),
        1..5
      ),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let list: LinkedList<i64> = xs.iter().cloned().collect();
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      // Create move closures to avoid borrowing
      let f_captured = f;
      let g_captured = g;

      // Apply f then g
      let result1 = list.clone().map(f_captured).map(g_captured);

      // Apply composition of f and g
      let result2 = list.clone().map(move |x| g_captured(&f_captured(x)));

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
      let list: LinkedList<i64> = xs.iter().cloned().collect();
      let f = INT_FUNCTIONS[f_idx];
      let result = list.clone().map(f);

      // Verify element-wise mapping
      prop_assert_eq!(result.len(), list.len());

      // Convert both to vectors for easier comparison
      let result_vec: Vec<_> = result.into_iter().collect();
      let expected_vec: Vec<_> = xs.iter().map(f).collect();

      prop_assert_eq!(result_vec, expected_vec);
    }
  }
}
