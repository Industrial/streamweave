use crate::{traits::category::Category, types::threadsafe::CloneableThreadSafe};
use std::sync::Arc;

// A cloneable function wrapper for Result
#[derive(Clone)]
pub struct ResultFn<A, B, E>(Arc<dyn Fn(Result<A, E>) -> Result<B, E> + Send + Sync + 'static>);

impl<A, B, E> ResultFn<A, B, E> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(Result<A, E>) -> Result<B, E> + Send + Sync + 'static,
  {
    ResultFn(Arc::new(f))
  }

  pub fn apply(&self, a: Result<A, E>) -> Result<B, E> {
    (self.0)(a)
  }
}

impl<T: CloneableThreadSafe, E: CloneableThreadSafe> Category<T, T> for Result<T, E> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = ResultFn<A, B, E>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    ResultFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    ResultFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(&A) -> B + CloneableThreadSafe,
  {
    ResultFn::new(move |x| x.map(|a| f(&a)))
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    ResultFn::new(move |x: Result<(A, C), E>| x.and_then(|(a, c)| f.apply(Ok(a)).map(|b| (b, c))))
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    ResultFn::new(move |x: Result<(C, A), E>| x.and_then(|(c, a)| f.apply(Ok(a)).map(|b| (c, b))))
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

  // Helper function to create a Result for testing
  fn make_result<T>(value: T, err: String, is_ok: bool) -> Result<T, String> {
    if is_ok {
      Ok(value)
    } else {
      Err(err)
    }
  }

  // Test the identity law: id() . f = f = f . id()
  proptest! {
      #[test]
      fn test_identity_law(
          // Use bounded input to prevent overflow
          x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          err in any::<String>(),
          is_ok in any::<bool>(),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create our Category::arr version of the function
          let arr_f = <Result<i64, String> as Category<i64, i64>>::arr(f);

          // Get the identity morphism
          let id = <Result<i64, String> as Category<i64, i64>>::id();

          // Compose id . f
          let id_then_f = <Result<i64, String> as Category<i64, i64>>::compose(id.clone(), arr_f.clone());

          // Compose f . id
          let f_then_id = <Result<i64, String> as Category<i64, i64>>::compose(arr_f.clone(), id);

          // Create a Result input
          let input = make_result(x, err.clone(), is_ok);

          // Apply the input to each composition
          let result_f = arr_f.apply(input.clone());
          let result_id_then_f = id_then_f.apply(input.clone());
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
          err in any::<String>(),
          is_ok in any::<bool>(),
          f_idx in 0..INT_FUNCTIONS.len(),
          g_idx in 0..INT_FUNCTIONS.len(),
          h_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get functions from our array
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <Result<i64, String> as Category<i64, i64>>::arr(f);
          let arr_g = <Result<i64, String> as Category<i64, i64>>::arr(g);
          let arr_h = <Result<i64, String> as Category<i64, i64>>::arr(h);

          // Create a Result input
          let input = make_result(x, err.clone(), is_ok);

          // Compose (f . g) . h
          let fg = <Result<i64, String> as Category<i64, i64>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <Result<i64, String> as Category<i64, i64>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <Result<i64, String> as Category<i64, i64>>::compose(arr_g, arr_h);
          let f_gh = <Result<i64, String> as Category<i64, i64>>::compose(arr_f, gh);

          // Apply the input
          let result_fg_h = fg_h.apply(input.clone());
          let result_f_gh = f_gh.apply(input);

          // Both compositions should give the same result
          assert_eq!(result_fg_h, result_f_gh);
      }
  }

  // Test the first combinator - specific to Result implementation (error handling)
  proptest! {
      #[test]
      fn test_first_combinator(
          // Use bounded input to prevent overflow
          x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          c in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          err in any::<String>(),
          is_ok in any::<bool>(),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Result<i64, String> as Category<i64, i64>>::arr(f);

          // Apply first to get a function on pairs
          let first_f = <Result<i64, String> as Category<i64, i64>>::first(arr_f);

          // Create a result pair input
          let pair = make_result((x, c), err.clone(), is_ok);

          // Apply the first combinator
          let result = first_f.apply(pair.clone());

          // The expected result
          let expected = pair.map(|(a, c)| (f(&a), c));

          // Results should match
          assert_eq!(result, expected);
      }
  }

  // Test the second combinator - specific to Result implementation (error handling)
  proptest! {
      #[test]
      fn test_second_combinator(
          // Use bounded input to prevent overflow
          x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          c in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          err in any::<String>(),
          is_ok in any::<bool>(),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Result<i64, String> as Category<i64, i64>>::arr(f);

          // Apply second to get a function on pairs
          let second_f = <Result<i64, String> as Category<i64, i64>>::second(arr_f);

          // Create a result pair input
          let pair = make_result((c, x), err.clone(), is_ok);

          // Apply the second combinator
          let result = second_f.apply(pair.clone());

          // The expected result
          let expected = pair.map(|(c, a)| (c, f(&a)));

          // Results should match
          assert_eq!(result, expected);
      }
  }
}
