use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe, E: CloneableThreadSafe> Functor<T> for Result<T, E> {
  type HigherSelf<U: CloneableThreadSafe> = Result<U, E>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    self.map(|x| f(&x))
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

  // Sample error types for Result tests
  #[derive(Debug, Clone, PartialEq)]
  enum TestError {
    Error1,
    Error2,
  }

  #[test]
  fn test_identity_law_ok() {
    // Choose a reasonable value for testing
    let x = 42i64;
    let result: Result<i64, TestError> = Ok(x);
    let id = |x: &i64| *x;
    // Use our Functor trait's map implementation
    let mapped = Functor::map(result, id);

    // Identity law: functor.map(id) == functor
    assert_eq!(mapped, Ok(x));
  }

  #[test]
  fn test_identity_law_err() {
    let result: Result<i64, TestError> = Err(TestError::Error1);
    let id = |x: &i64| *x;
    // Use our Functor trait's map implementation
    let mapped = Functor::map(result, id);

    // Identity law: functor.map(id) == functor
    assert_eq!(mapped, Err(TestError::Error1));
  }

  proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn test_identity_law_ok_prop(x in any::<i64>().prop_filter("Value too large", |v| *v < 10000)) {
      let result: Result<i64, TestError> = Ok(x);
      let id = |x: &i64| *x;
      // Use our Functor trait's map implementation
      let mapped = Functor::map(result, id);

      // Identity law: functor.map(id) == functor
      prop_assert_eq!(mapped, Ok(x));
    }

    #[test]
    fn test_composition_law_ok(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];
      let result: Result<i64, TestError> = Ok(x);

      // Apply f then g using our Functor trait's map implementation
      let result1 = Functor::map(result.clone(), f);
      let result1 = Functor::map(result1, g);

      // Apply composition of f and g
      let composed = move |x: &i64| g(&f(x));
      let result2 = Functor::map(result, composed);

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn test_composition_law_err(
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];
      let result: Result<i64, TestError> = Err(TestError::Error2);

      // Apply f then g using our Functor trait's map implementation
      let result1 = Functor::map(result.clone(), f);
      let result1 = Functor::map(result1, g);

      // Apply composition of f and g
      let composed = move |x: &i64| g(&f(x));
      let result2 = Functor::map(result, composed);

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }

    // Implementation-specific test: error propagation
    #[test]
    fn test_error_propagation(f_idx in 0..INT_FUNCTIONS.len()) {
      let f = INT_FUNCTIONS[f_idx];
      let result: Result<i64, TestError> = Err(TestError::Error1);

      // Mapping over Err should preserve the Err structure and error value
      let mapped = Functor::map(result, f);
      prop_assert_eq!(mapped, Err(TestError::Error1));
    }
  }
}
