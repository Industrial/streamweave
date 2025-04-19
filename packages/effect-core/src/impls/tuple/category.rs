use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;
use std::marker::PhantomData;
use std::sync::Arc;

// Define a morphism for tuples
#[derive(Clone)]
pub struct TupleMorphism<A, B, C>(Arc<dyn Fn(B) -> C + Send + Sync + 'static>, PhantomData<A>);

impl<A, B, C> TupleMorphism<A, B, C> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(B) -> C + Send + Sync + 'static,
  {
    TupleMorphism(Arc::new(f), PhantomData)
  }

  pub fn apply(&self, input: B) -> C {
    (self.0)(input)
  }
}

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Category<B, B> for (A, B) {
  type Morphism<X: CloneableThreadSafe, Y: CloneableThreadSafe> = TupleMorphism<A, X, Y>;

  fn id<X: CloneableThreadSafe>() -> Self::Morphism<X, X> {
    TupleMorphism::new(|x| x)
  }

  fn compose<X: CloneableThreadSafe, Y: CloneableThreadSafe, Z: CloneableThreadSafe>(
    f: Self::Morphism<X, Y>,
    g: Self::Morphism<Y, Z>,
  ) -> Self::Morphism<X, Z> {
    TupleMorphism::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<X: CloneableThreadSafe, Y: CloneableThreadSafe, F>(f: F) -> Self::Morphism<X, Y>
  where
    F: for<'a> Fn(&'a X) -> Y + CloneableThreadSafe,
  {
    TupleMorphism::new(move |x| f(&x))
  }

  fn first<X: CloneableThreadSafe, Y: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<X, Y>,
  ) -> Self::Morphism<(X, C), (Y, C)> {
    TupleMorphism::new(move |(x, c)| (f.apply(x), c))
  }

  fn second<X: CloneableThreadSafe, Y: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<X, Y>,
  ) -> Self::Morphism<(C, X), (C, Y)> {
    TupleMorphism::new(move |(c, x)| (c, f.apply(x)))
  }
}

#[cfg(test)]
mod tests {
  use crate::traits::category::Category;
  use proptest::prelude::*;

  // Define test functions with overflow protection for property-based testing
  const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  // Test the identity law: id() . f = f = f . id()
  #[test]
  fn test_identity_law() {
    let x = 42i64;

    (0..INT_FUNCTIONS.len()).for_each(|f_idx| {
      let f = INT_FUNCTIONS[f_idx];

      // Create our Category::arr version of the function
      let arr_f = <(i32, i64) as Category<i64, i64>>::arr(f);

      // Get the identity morphism
      let id = <(i32, i64) as Category<i64, i64>>::id();

      // Compose id . f
      let id_then_f = <(i32, i64) as Category<i64, i64>>::compose(id.clone(), arr_f.clone());

      // Compose f . id
      let f_then_id = <(i32, i64) as Category<i64, i64>>::compose(arr_f.clone(), id);

      // Apply the input to each
      let result_f = arr_f.apply(x);
      let result_id_then_f = id_then_f.apply(x);
      let result_f_then_id = f_then_id.apply(x);

      // All should give the same result
      assert_eq!(result_f, result_id_then_f);
      assert_eq!(result_f, result_f_then_id);
    });
  }

  // Property-based test for identity law
  proptest! {
    #[test]
    fn test_identity_law_prop(
      x in -1000..1000i64,
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];

      // Create our Category::arr version of the function
      let arr_f = <(i32, i64) as Category<i64, i64>>::arr(f);

      // Get the identity morphism
      let id = <(i32, i64) as Category<i64, i64>>::id();

      // Compose id . f
      let id_then_f = <(i32, i64) as Category<i64, i64>>::compose(id.clone(), arr_f.clone());

      // Compose f . id
      let f_then_id = <(i32, i64) as Category<i64, i64>>::compose(arr_f.clone(), id);

      // Apply the input to each
      let result_f = arr_f.apply(x);
      let result_id_then_f = id_then_f.apply(x);
      let result_f_then_id = f_then_id.apply(x);

      // All should give the same result
      prop_assert_eq!(result_f, result_id_then_f);
      prop_assert_eq!(result_f, result_f_then_id);
    }
  }

  // Test the composition law: (f . g) . h = f . (g . h)
  #[test]
  fn test_composition_law() {
    let x = 42i64;

    (0..INT_FUNCTIONS.len()).for_each(|f_idx| {
      (0..INT_FUNCTIONS.len()).for_each(|g_idx| {
        (0..INT_FUNCTIONS.len()).for_each(|h_idx| {
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <(i32, i64) as Category<i64, i64>>::arr(f);
          let arr_g = <(i32, i64) as Category<i64, i64>>::arr(g);
          let arr_h = <(i32, i64) as Category<i64, i64>>::arr(h);

          // Compose (f . g) . h
          let fg = <(i32, i64) as Category<i64, i64>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <(i32, i64) as Category<i64, i64>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <(i32, i64) as Category<i64, i64>>::compose(arr_g, arr_h);
          let f_gh = <(i32, i64) as Category<i64, i64>>::compose(arr_f, gh);

          // Apply the input to each
          let result_fg_h = fg_h.apply(x);
          let result_f_gh = f_gh.apply(x);

          // Both compositions should give the same result
          assert_eq!(result_fg_h, result_f_gh);
        });
      });
    });
  }

  // Property-based test for composition law (with a subset of combinations to keep runtime reasonable)
  proptest! {
    #[test]
    fn test_composition_law_prop(
      x in -1000..1000i64,
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len(),
      h_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];
      let h = INT_FUNCTIONS[h_idx];

      // Create Category::arr versions
      let arr_f = <(i32, i64) as Category<i64, i64>>::arr(f);
      let arr_g = <(i32, i64) as Category<i64, i64>>::arr(g);
      let arr_h = <(i32, i64) as Category<i64, i64>>::arr(h);

      // Compose (f . g) . h
      let fg = <(i32, i64) as Category<i64, i64>>::compose(arr_f.clone(), arr_g.clone());
      let fg_h = <(i32, i64) as Category<i64, i64>>::compose(fg, arr_h.clone());

      // Compose f . (g . h)
      let gh = <(i32, i64) as Category<i64, i64>>::compose(arr_g, arr_h);
      let f_gh = <(i32, i64) as Category<i64, i64>>::compose(arr_f, gh);

      // Apply the input to each
      let result_fg_h = fg_h.apply(x);
      let result_f_gh = f_gh.apply(x);

      // Both compositions should give the same result
      prop_assert_eq!(result_fg_h, result_f_gh);
    }
  }

  // Test the first combinator
  #[test]
  fn test_first_combinator() {
    let x = 42i64;
    let c = 10i64;

    (0..INT_FUNCTIONS.len()).for_each(|f_idx| {
      let f = INT_FUNCTIONS[f_idx];

      // Create the arr version
      let arr_f = <(i32, i64) as Category<i64, i64>>::arr(f);

      // Apply first to get a function on pairs
      let first_f = <(i32, i64) as Category<i64, i64>>::first(arr_f);

      // Apply the first combinator
      let result = first_f.apply((x, c));

      // The expected result is (f(x), c)
      let expected = (f(&x), c);

      // Results should match
      assert_eq!(result, expected);
    });
  }

  // Test the second combinator
  #[test]
  fn test_second_combinator() {
    let x = 42i64;
    let c = 10i64;

    (0..INT_FUNCTIONS.len()).for_each(|f_idx| {
      let f = INT_FUNCTIONS[f_idx];

      // Create the arr version
      let arr_f = <(i32, i64) as Category<i64, i64>>::arr(f);

      // Apply second to get a function on pairs
      let second_f = <(i32, i64) as Category<i64, i64>>::second(arr_f);

      // Apply the second combinator
      let result = second_f.apply((c, x));

      // The expected result is (c, f(x))
      let expected = (c, f(&x));

      // Results should match
      assert_eq!(result, expected);
    });
  }

  // Test the first/second commutativity law: first(f) . second(g) = second(g) . first(f)
  #[test]
  fn test_first_second_commutativity() {
    let pair = (42i64, 10i64);

    (0..INT_FUNCTIONS.len()).for_each(|f_idx| {
      (0..INT_FUNCTIONS.len()).for_each(|g_idx| {
        let f = INT_FUNCTIONS[f_idx];
        let g = INT_FUNCTIONS[g_idx];

        // Create the arr versions
        let arr_f = <(i32, i64) as Category<i64, i64>>::arr(f);
        let arr_g = <(i32, i64) as Category<i64, i64>>::arr(g);

        // Create first(f) and second(g)
        let first_f = <(i32, i64) as Category<i64, i64>>::first(arr_f.clone());
        let second_g = <(i32, i64) as Category<i64, i64>>::second(arr_g.clone());

        // Compose first(f) . second(g)
        let first_f_then_second_g =
          <(i32, (i64, i64)) as Category<(i64, i64), (i64, i64)>>::compose(
            first_f.clone(),
            second_g.clone(),
          );

        // Compose second(g) . first(f)
        let second_g_then_first_f =
          <(i32, (i64, i64)) as Category<(i64, i64), (i64, i64)>>::compose(second_g, first_f);

        // Apply both compositions to the pair
        let result1 = first_f_then_second_g.apply(pair);
        let result2 = second_g_then_first_f.apply(pair);

        // Results should be the same
        assert_eq!(result1, result2);
      });
    });
  }

  // Test with different types that conform to the Category<B, B> constraint
  #[test]
  fn test_category_with_different_types() {
    // Test with an UpperCase function that keeps the same type (String -> String)
    let to_uppercase = |s: &String| s.to_uppercase();

    let arr_uppercase = <(i32, String) as Category<String, String>>::arr(to_uppercase);

    let result = arr_uppercase.apply("hello".to_string());
    assert_eq!(result, "HELLO");

    // Test with an numeric operations (i64 -> i64)
    let double = |x: &i64| x * 2;
    let add_one = |x: &i64| x + 1;

    let arr_double = <(i32, i64) as Category<i64, i64>>::arr(double);
    let arr_add_one = <(i32, i64) as Category<i64, i64>>::arr(add_one);

    let composed = <(i32, i64) as Category<i64, i64>>::compose(arr_double, arr_add_one);

    let result = composed.apply(5);
    assert_eq!(result, 11); // 5 * 2 + 1 = 11
  }

  // Test with complex nested pairs
  #[test]
  fn test_category_with_nested_pairs() {
    let increment = |x: &i64| x + 1;
    let double = |x: &i64| x * 2;

    let arr_increment = <(i32, i64) as Category<i64, i64>>::arr(increment);
    let arr_double = <(i32, i64) as Category<i64, i64>>::arr(double);

    // Apply first to get a function on pairs
    let first_increment = <(i32, i64) as Category<i64, i64>>::first(arr_increment.clone());

    // Apply first again to get a function on nested pairs
    let first_first_increment =
      <(i32, (i64, i64)) as Category<(i64, i64), (i64, i64)>>::first(first_increment.clone());

    // Apply second to get a function on pairs
    let second_double = <(i32, i64) as Category<i64, i64>>::second(arr_double);

    // Apply the nested combinators to a nested pair
    let nested_pair = ((5, 10), 20);
    let result = first_first_increment.apply(nested_pair);

    assert_eq!(result, ((6, 10), 20));

    // Test composition with nested pairs
    let first_increment_then_second_double = <(i32, (i64, i64)) as Category<
      (i64, i64),
      (i64, i64),
    >>::compose(first_increment, second_double);

    let pair = (5, 10);
    let result = first_increment_then_second_double.apply(pair);

    assert_eq!(result, (6, 20));
  }
}
