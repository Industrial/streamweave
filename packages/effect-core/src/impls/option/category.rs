use crate::{traits::category::Category, types::threadsafe::CloneableThreadSafe};
use std::sync::Arc;

// A cloneable function wrapper for Option
#[derive(Clone)]
pub struct OptionFn<A, B>(Arc<dyn Fn(Option<A>) -> Option<B> + Send + Sync + 'static>);

impl<A, B> OptionFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(Option<A>) -> Option<B> + Send + Sync + 'static,
  {
    OptionFn(Arc::new(f))
  }

  pub fn apply(&self, a: Option<A>) -> Option<B> {
    (self.0)(a)
  }
}

impl<T: CloneableThreadSafe> Category<T, T> for Option<T> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = OptionFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    OptionFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    OptionFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(&A) -> B + CloneableThreadSafe,
  {
    OptionFn::new(move |x| x.map(|a| f(&a)))
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    OptionFn::new(move |x: Option<(A, C)>| {
      x.and_then(|(a, c)| {
        let b = f.apply(Some(a));
        b.map(|b_val| (b_val, c))
      })
    })
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    OptionFn::new(move |x: Option<(C, A)>| {
      x.and_then(|(c, a)| {
        let b = f.apply(Some(a));
        b.map(|b_val| (c, b_val))
      })
    })
  }
}

#[cfg(test)]
mod tests {
  use crate::traits::category::Category;
  use proptest::prelude::*;

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

  // Helper function to create an Option for testing
  fn make_option<T>(value: T, is_some: bool) -> Option<T> {
    if is_some {
      Some(value)
    } else {
      None
    }
  }

  // Test the identity law: id() . f = f = f . id()
  proptest! {
      #[test]
      fn test_identity_law(
          // Use bounded input to prevent overflow
          x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          is_some in any::<bool>(),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create our Category::arr version of the function
          let arr_f = <Option<i64> as Category<i64, i64>>::arr(f);

          // Get the identity morphism
          let id = <Option<i64> as Category<i64, i64>>::id();

          // Compose id . f
          let id_then_f = <Option<i64> as Category<i64, i64>>::compose(id.clone(), arr_f.clone());

          // Compose f . id
          let f_then_id = <Option<i64> as Category<i64, i64>>::compose(arr_f.clone(), id);

          // Create an Option input
          let input = make_option(x, is_some);

          // Apply the input to each composition
          let result_f = arr_f.apply(input);
          let result_id_then_f = id_then_f.apply(input);
          let result_f_then_id = f_then_id.apply(input);

          // All should give the same result
          assert_eq!(result_f, result_id_then_f);
          assert_eq!(result_f, result_f_then_id);
      }
  }

  // Test the composition law: (f . g) . h = f . (g . h)
  proptest! {
      #[test]
      fn test_composition_law(
          // Use bounded input to prevent overflow
          x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          is_some in any::<bool>(),
          f_idx in 0..INT_FUNCTIONS.len(),
          g_idx in 0..INT_FUNCTIONS.len(),
          h_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get functions from our array
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <Option<i64> as Category<i64, i64>>::arr(f);
          let arr_g = <Option<i64> as Category<i64, i64>>::arr(g);
          let arr_h = <Option<i64> as Category<i64, i64>>::arr(h);

          // Create an Option input
          let input = make_option(x, is_some);

          // Compose (f . g) . h
          let fg = <Option<i64> as Category<i64, i64>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <Option<i64> as Category<i64, i64>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <Option<i64> as Category<i64, i64>>::compose(arr_g, arr_h);
          let f_gh = <Option<i64> as Category<i64, i64>>::compose(arr_f, gh);

          // Apply the input
          let result_fg_h = fg_h.apply(input);
          let result_f_gh = f_gh.apply(input);

          // Both compositions should give the same result
          assert_eq!(result_fg_h, result_f_gh);
      }
  }

  // Test the first combinator - specific to Option implementation (None handling)
  proptest! {
      #[test]
      fn test_first_combinator(
          // Use bounded input to prevent overflow
          x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          c in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          is_some in any::<bool>(),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Option<i64> as Category<i64, i64>>::arr(f);

          // Apply first to get a function on pairs
          let first_f = <Option<i64> as Category<i64, i64>>::first(arr_f);

          // Create an option pair input
          let pair = make_option((x, c), is_some);

          // Apply the first combinator
          let result = first_f.apply(pair);

          // The expected result
          let expected = pair.map(|(a, c)| (f(&a), c));

          // Results should match
          assert_eq!(result, expected);
      }
  }

  // Test the second combinator - specific to Option implementation (None handling)
  proptest! {
      #[test]
      fn test_second_combinator(
          // Use bounded input to prevent overflow
          x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          c in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          is_some in any::<bool>(),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Option<i64> as Category<i64, i64>>::arr(f);

          // Apply second to get a function on pairs
          let second_f = <Option<i64> as Category<i64, i64>>::second(arr_f);

          // Create an option pair input
          let pair = make_option((c, x), is_some);

          // Apply the second combinator
          let result = second_f.apply(pair);

          // The expected result
          let expected = pair.map(|(c, a)| (c, f(&a)));

          // Results should match
          assert_eq!(result, expected);
      }
  }
}
