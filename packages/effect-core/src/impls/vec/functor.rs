use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Functor<T> for Vec<T> {
  type HigherSelf<U: CloneableThreadSafe> = Vec<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
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
  use std::fmt::Debug;

  // Define test functions with overflow protection
  const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  // Helper for creating readable function names in test output
  fn function_name(idx: usize) -> &'static str {
    match idx {
      0 => "add1",
      1 => "mul2",
      2 => "sub1",
      3 => "div2",
      4 => "square",
      5 => "negate",
      _ => "unknown",
    }
  }

  #[test]
  fn test_identity_law_empty() {
    let vec: Vec<i64> = vec![];
    let id = |x: &i64| *x;
    let result = vec.map(id);

    // Identity law for empty vector
    assert_eq!(result, Vec::<i64>::new());
  }

  #[test]
  fn test_identity_law_nonempty() {
    let xs = vec![1, 2, 3];
    let vec = xs.clone();
    let id = |x: &i64| *x;
    let result = vec.map(id);

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
      let vec = xs.clone();
      let id = |x: &i64| *x;
      let result = vec.map(id);

      // Identity law: functor.map(id) == functor
      prop_assert_eq!(result, xs);
    }

    #[test]
    fn test_composition_law_empty(
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];
      let vec: Vec<i64> = vec![];

      // Create move closures to avoid borrowing
      let f_captured = f;
      let g_captured = g;

      // Apply f then g
      let result1 = vec.clone().map(f_captured).map(g_captured);

      // Apply composition of f and g
      let result2 = vec.map(move |x| g_captured(&f_captured(x)));

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
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      // Create move closures to avoid borrowing
      let f_captured = f;
      let g_captured = g;

      // Apply f then g
      let result1 = xs.clone().map(f_captured).map(g_captured);

      // Apply composition of f and g
      let result2 = xs.clone().map(move |x| g_captured(&f_captured(x)));

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }

    // Implementation-specific test: vector element-wise operations
    #[test]
    fn test_element_wise_mapping(
      xs in prop::collection::vec(
        any::<i64>().prop_filter("Value too large", |v| *v < 10000),
        1..5
      ),
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let result = xs.clone().map(f);

      // Verify element-wise mapping
      prop_assert_eq!(result.len(), xs.len());

      for (i, x) in xs.iter().enumerate() {
        prop_assert_eq!(result[i], f(x));
      }
    }
  }
}
