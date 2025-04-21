use crate::impls::vecdeque::category::VecDequeCategory;
use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::VecDeque;

// Implement Functor for VecDequeCategory, not for VecDeque directly
impl<T: CloneableThreadSafe> Functor<T> for VecDequeCategory {
  type HigherSelf<U: CloneableThreadSafe> = VecDequeCategory;

  fn map<U, F>(self, _f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    // This is a placeholder implementation since VecDequeCategory is just a proxy type
    // The actual mapping happens via the Category impl when using arr and apply
    VecDequeCategory
  }
}

// Extension trait to make VecDeque mapping more ergonomic
pub trait VecDequeFunctorExt<T: CloneableThreadSafe> {
  fn map<U, F>(self, f: F) -> VecDeque<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe;
}

// Implement the extension trait for VecDeque
impl<T: CloneableThreadSafe> VecDequeFunctorExt<T> for VecDeque<T> {
  fn map<U, F>(self, mut f: F) -> VecDeque<U>
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

  // Helper function to convert a Vec to a VecDeque
  fn to_vecdeque<T: Clone>(v: Vec<T>) -> VecDeque<T> {
    v.into_iter().collect()
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
    let vecdeque: VecDeque<i64> = VecDeque::new();
    let id = |x: &i64| *x;
    let result = vecdeque.map(id);

    // Identity law for empty vector
    assert_eq!(result, VecDeque::<i64>::new());
  }

  #[test]
  fn test_identity_law_nonempty() {
    let xs = to_vecdeque(vec![1, 2, 3]);
    let vecdeque = xs.clone();
    let id = |x: &i64| *x;
    let result = vecdeque.map(id);

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
      let vecdeque = to_vecdeque(xs.clone());
      let id = |x: &i64| *x;
      let result = vecdeque.map(id);

      // Identity law: functor.map(id) == functor
      prop_assert_eq!(result, to_vecdeque(xs));
    }

    #[test]
    fn test_composition_law_empty(
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];
      let vecdeque: VecDeque<i64> = VecDeque::new();

      // Create move closures to avoid borrowing
      let f_captured = f;
      let g_captured = g;

      // Apply f then g
      let result1 = vecdeque.clone().map(f_captured).map(g_captured);

      // Apply composition of f and g
      let result2 = vecdeque.map(move |x| g_captured(&f_captured(x)));

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
      let vecdeque = to_vecdeque(xs.clone());
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      // Create move closures to avoid borrowing
      let f_captured = f;
      let g_captured = g;

      // Apply f then g
      let result1 = vecdeque.clone().map(f_captured).map(g_captured);

      // Apply composition of f and g
      let result2 = vecdeque.clone().map(move |x| g_captured(&f_captured(x)));

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
      let vecdeque = to_vecdeque(xs.clone());
      let f = INT_FUNCTIONS[f_idx];
      let result = vecdeque.clone().map(f);

      // Verify element-wise mapping
      prop_assert_eq!(result.len(), vecdeque.len());

      for (i, x) in xs.iter().enumerate() {
        prop_assert_eq!(result.get(i).unwrap(), &f(x));
      }
    }
  }
}
