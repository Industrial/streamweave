// Implementation has been removed due to type system compatibility issues
// We will need to revisit the Bifunctor trait design

use crate::traits::bifunctor::Bifunctor;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Bifunctor<A, B> for (A, B) {
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe> = (C, D);

  fn bimap<C, D, F, G>(self, mut f: F, mut g: G) -> Self::HigherSelf<C, D>
  where
    F: for<'a> FnMut(&'a A) -> C + CloneableThreadSafe,
    G: for<'b> FnMut(&'b B) -> D + CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    (f(&self.0), g(&self.1))
  }

  fn first<C, F>(self, mut f: F) -> Self::HigherSelf<C, B>
  where
    F: for<'a> FnMut(&'a A) -> C + CloneableThreadSafe,
    C: CloneableThreadSafe,
  {
    (f(&self.0), self.1)
  }

  fn second<D, G>(self, mut g: G) -> Self::HigherSelf<A, D>
  where
    G: for<'b> FnMut(&'b B) -> D + CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    (self.0, g(&self.1))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test functions with overflow protection for property-based testing
  const INT_FUNCTIONS: &[fn(&i32) -> i32] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.saturating_neg(),
  ];

  // Test identity law: bimap(id, id) = id
  #[test]
  fn test_identity_law() {
    let pair = (42, 84);
    let id_a = |x: &i32| *x;
    let id_b = |x: &i32| *x;

    let result = pair.bimap(id_a, id_b);
    assert_eq!(result, pair);
  }

  // Property-based test for identity law
  proptest! {
    #[test]
    fn test_identity_law_prop(a in -1000..1000i32, b in -1000..1000i32) {
      let pair = (a, b);
      let id_a = |x: &i32| *x;
      let id_b = |x: &i32| *x;

      let result = pair.bimap(id_a, id_b);
      prop_assert_eq!(result, pair);
    }
  }

  // Test composition law: bimap(f1, g1) . bimap(f2, g2) = bimap(f1 . f2, g1 . g2)
  #[test]
  fn test_composition_law() {
    let pair = (5, 10);

    // Test with all combinations of our test functions
    (0..INT_FUNCTIONS.len()).for_each(|f1_idx| {
      (0..INT_FUNCTIONS.len()).for_each(|f2_idx| {
        (0..INT_FUNCTIONS.len()).for_each(|g1_idx| {
          (0..INT_FUNCTIONS.len()).for_each(|g2_idx| {
            let f1 = INT_FUNCTIONS[f1_idx];
            let f2 = INT_FUNCTIONS[f2_idx];
            let g1 = INT_FUNCTIONS[g1_idx];
            let g2 = INT_FUNCTIONS[g2_idx];

            // First approach: apply bimap twice
            let intermediate = pair.bimap(f1, g1);
            let result1 = intermediate.bimap(f2, g2);

            // Second approach: compose the functions, then apply bimap once
            let f_composed = move |x: &i32| f2(&f1(x));
            let g_composed = move |x: &i32| g2(&g1(x));

            // Create simple mapping wrappers that satisfy the trait bounds
            let f_wrapper = move |x: &i32| f_composed(x);
            let g_wrapper = move |x: &i32| g_composed(x);

            let result2 = pair.bimap(f_wrapper, g_wrapper);

            // Results should be the same
            assert_eq!(result1, result2);
          });
        });
      });
    });
  }

  // Property-based test for composition law (with a subset of combinations to keep runtime reasonable)
  proptest! {
    #[test]
    fn test_composition_law_prop(
      a in -100..100i32,
      b in -100..100i32,
      f1_idx in 0..INT_FUNCTIONS.len(),
      f2_idx in 0..INT_FUNCTIONS.len(),
      g1_idx in 0..INT_FUNCTIONS.len(),
      g2_idx in 0..INT_FUNCTIONS.len()
    ) {
      let pair = (a, b);
      let f1 = INT_FUNCTIONS[f1_idx];
      let f2 = INT_FUNCTIONS[f2_idx];
      let g1 = INT_FUNCTIONS[g1_idx];
      let g2 = INT_FUNCTIONS[g2_idx];

      // First approach: apply bimap twice
      let intermediate = pair.bimap(f1, g1);
      let result1 = intermediate.bimap(f2, g2);

      // Second approach: compose the functions, then apply bimap once
      let f_composed = move |x: &i32| f2(&f1(x));
      let g_composed = move |x: &i32| g2(&g1(x));

      // Create simple mapping wrappers that satisfy the trait bounds
      let f_wrapper = move |x: &i32| f_composed(x);
      let g_wrapper = move |x: &i32| g_composed(x);

      let result2 = pair.bimap(f_wrapper, g_wrapper);

      // Results should be the same
      prop_assert_eq!(result1, result2);
    }
  }

  // Test that first and second are derived from bimap correctly
  #[test]
  fn test_first_second_derivation() {
    let pair = (5, 10);

    // For all our test functions
    (0..INT_FUNCTIONS.len()).for_each(|f_idx| {
      let f = INT_FUNCTIONS[f_idx];

      // First using the dedicated method
      let result1 = pair.first(f);

      // First as a special case of bimap
      let id_b = |x: &i32| *x;
      let result2 = pair.bimap(f, id_b);

      // Results should be the same
      assert_eq!(result1, result2);

      // Second using the dedicated method
      let result3 = pair.second(f);

      // Second as a special case of bimap
      let id_a = |x: &i32| *x;
      let result4 = pair.bimap(id_a, f);

      // Results should be the same
      assert_eq!(result3, result4);
    });
  }

  // Property-based test for first/second derivation
  proptest! {
    #[test]
    fn test_first_second_derivation_prop(
      a in -1000..1000i32,
      b in -1000..1000i32,
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let pair = (a, b);
      let f = INT_FUNCTIONS[f_idx];

      // First using the dedicated method
      let result1 = pair.first(f);

      // First as a special case of bimap
      let id_b = |x: &i32| *x;
      let result2 = pair.bimap(f, id_b);

      // Results should be the same
      prop_assert_eq!(result1, result2);

      // Second using the dedicated method
      let result3 = pair.second(f);

      // Second as a special case of bimap
      let id_a = |x: &i32| *x;
      let result4 = pair.bimap(id_a, f);

      // Results should be the same
      prop_assert_eq!(result3, result4);
    }
  }

  // Test first/second commutativity law: first(f) . second(g) = second(g) . first(f)
  #[test]
  fn test_first_second_commutativity() {
    let pair = (5, 10);

    // Test with all combinations of our test functions
    (0..INT_FUNCTIONS.len()).for_each(|f_idx| {
      (0..INT_FUNCTIONS.len()).for_each(|g_idx| {
        let f = INT_FUNCTIONS[f_idx];
        let g = INT_FUNCTIONS[g_idx];

        // First approach: first(f) then second(g)
        let first_then_second = pair.first(f).second(g);

        // Second approach: second(g) then first(f)
        let second_then_first = pair.second(g).first(f);

        // Results should be the same
        assert_eq!(first_then_second, second_then_first);
      });
    });
  }

  // Property-based test for first/second commutativity
  proptest! {
    #[test]
    fn test_first_second_commutativity_prop(
      a in -1000..1000i32,
      b in -1000..1000i32,
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let pair = (a, b);
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      // First approach: first(f) then second(g)
      let first_then_second = pair.first(f).second(g);

      // Second approach: second(g) then first(f)
      let second_then_first = pair.second(g).first(f);

      // Results should be the same
      prop_assert_eq!(first_then_second, second_then_first);
    }
  }

  // Test with different types
  #[test]
  fn test_bifunctor_with_different_types() {
    // String -> Length, Number -> Double
    let pair = ("hello", 5);
    let string_len = |s: &&str| s.len();
    let double = |n: &i32| n * 2;

    let result = pair.bimap(string_len, double);
    assert_eq!(result, (5, 10));

    // Only transform first component
    let result = pair.first(string_len);
    assert_eq!(result, (5, 5));

    // Only transform second component
    let result = pair.second(double);
    assert_eq!(result, ("hello", 10));
  }

  // Test with complex types
  #[test]
  fn test_bifunctor_with_complex_types() {
    // Option types
    let pair = (Some(5), Some(10));
    let option_map1 = |opt: &Option<i32>| opt.map(|x| x + 1);
    let option_map2 = |opt: &Option<i32>| opt.map(|x| x * 2);

    let result = pair.bimap(option_map1, option_map2);
    assert_eq!(result, (Some(6), Some(20)));

    // Mix of Option and Vec
    let pair = (Some(42), vec![1, 2, 3]);
    let option_is_some = |opt: &Option<i32>| opt.is_some();
    let vec_len = |v: &Vec<i32>| v.len();

    let result = pair.bimap(option_is_some, vec_len);
    assert_eq!(result, (true, 3));
  }

  // Test with nested tuples
  #[test]
  fn test_bifunctor_with_nested_tuples() {
    // The outer tuple contains an inner tuple as its first component
    let nested = ((1, 2), 3);

    // Map the first component (which is a tuple) using another bimap
    let result = nested.first(|inner_tuple| inner_tuple.bimap(|x| x + 1, |y| y * 2));
    assert_eq!(result, ((2, 4), 3));

    // The outer tuple contains an inner tuple as its second component
    let nested = (1, (2, 3));

    // Map the second component (which is a tuple) using another bimap
    let result = nested.second(|inner_tuple| inner_tuple.bimap(|x| x + 1, |y| y * 2));
    assert_eq!(result, (1, (3, 6)));
  }
}
