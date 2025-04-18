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

  // Functions that may return the same value for certain inputs
  const SPECIAL_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| if *x < 0 { 0 } else { *x },           // ReLU-like function
    |x| *x % 10,                               // Modulo function
    |x| if *x % 2 == 0 { *x } else { *x + 1 }, // Make even
  ];

  // Sample error types for Result tests
  #[derive(Debug, Clone, PartialEq)]
  enum TestError {
    Error1,
    Error2,
    CustomError(String),
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

  #[test]
  fn test_map_with_string() {
    // Test with a different type (String)
    let result: Result<&str, TestError> = Ok("hello");
    let f = |s: &&str| s.to_uppercase();
    let mapped = Functor::map(result, f);
    assert_eq!(mapped, Ok("HELLO".to_string()));

    // Test with error
    let result: Result<&str, TestError> = Err(TestError::Error2);
    let mapped = Functor::map(result, f);
    assert_eq!(mapped, Err(TestError::Error2));
  }

  #[test]
  fn test_map_with_struct() {
    // Test with a custom struct
    #[derive(Debug, PartialEq, Clone)]
    struct Person {
      name: String,
      age: u32,
    }

    let person = Person {
      name: "Alice".to_string(),
      age: 30,
    };
    let result: Result<Person, TestError> = Ok(person);

    let f = |p: &Person| p.age;
    let mapped = Functor::map(result, f);
    assert_eq!(mapped, Ok(30));

    // Test with error
    let result: Result<Person, TestError> =
      Err(TestError::CustomError("Person not found".to_string()));
    let mapped = Functor::map(result, f);
    assert_eq!(
      mapped,
      Err(TestError::CustomError("Person not found".to_string()))
    );
  }

  #[test]
  fn test_map_to_result() {
    // Test mapping to another Result type
    let result: Result<i64, TestError> = Ok(10);
    let f = |x: &i64| Result::<i64, String>::Ok(*x * 2);
    let mapped = Functor::map(result, f);
    assert_eq!(mapped, Ok(Result::<i64, String>::Ok(20)));

    // Test with error
    let result: Result<i64, TestError> = Err(TestError::Error1);
    let mapped = Functor::map(result, f);
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
      g_idx in 0..INT_FUNCTIONS.len(),
      error_idx in 0..2u8 // Use 0 for Error1 and 1 for Error2
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      // Create different error values for thorough testing
      let error = match error_idx {
        0 => TestError::Error1,
        _ => TestError::Error2,
      };

      let result: Result<i64, TestError> = Err(error.clone());

      // Apply f then g using our Functor trait's map implementation
      let result1 = Functor::map(result.clone(), f);
      let result1_then_g = Functor::map(result1, g);

      // Apply composition of f and g
      let composed = move |x: &i64| g(&f(x));
      let result2 = Functor::map(result, composed);

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1_then_g.clone(), result2);
      prop_assert_eq!(result1_then_g, Err(error));
    }

    // Test with special functions that may return the same value for different inputs
    #[test]
    fn test_composition_with_special_functions(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..SPECIAL_FUNCTIONS.len(),
      g_idx in 0..SPECIAL_FUNCTIONS.len()
    ) {
      let f = SPECIAL_FUNCTIONS[f_idx];
      let g = SPECIAL_FUNCTIONS[g_idx];
      let result: Result<i64, TestError> = Ok(x);

      // Apply f then g
      let result_f = Functor::map(result.clone(), f);
      let result_f_then_g = Functor::map(result_f, g);

      // Apply composition of f and g
      let composed = move |x: &i64| g(&f(x));
      let result_composed = Functor::map(result, composed);

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result_f_then_g, result_composed);
    }

    // Test with different integer values including edge cases
    #[test]
    fn test_with_edge_case_values(
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];

      // Edge case: i64::MAX
      let result: Result<i64, TestError> = Ok(i64::MAX);
      let mapped = Functor::map(result, f);
      prop_assert_eq!(mapped, Ok(f(&i64::MAX)));

      // Edge case: i64::MIN
      let result: Result<i64, TestError> = Ok(i64::MIN);
      let mapped = Functor::map(result, f);
      prop_assert_eq!(mapped, Ok(f(&i64::MIN)));

      // Edge case: 0
      let result: Result<i64, TestError> = Ok(0i64);
      let mapped = Functor::map(result, f);
      prop_assert_eq!(mapped, Ok(f(&0)));
    }

    // Implementation-specific test: error propagation with different error types
    #[test]
    fn test_error_propagation(
      f_idx in 0..INT_FUNCTIONS.len(),
      error_type in 0..3u8 // 0=Error1, 1=Error2, 2=CustomError
    ) {
      let f = INT_FUNCTIONS[f_idx];

      // Create different error types
      let error = match error_type {
        0 => TestError::Error1,
        1 => TestError::Error2,
        _ => TestError::CustomError("Test error message".to_string()),
      };

      let result: Result<i64, TestError> = Err(error.clone());

      // Mapping over Err should preserve the Err structure and error value
      let mapped = Functor::map(result, f);
      prop_assert_eq!(mapped, Err(error));
    }

    // Test with nested Result
    #[test]
    fn test_nested_result(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 100),
      is_outer_ok in any::<bool>(),
      is_inner_ok in any::<bool>()
    ) {
      // Create nested Result based on boolean flags
      let inner_result = if is_inner_ok {
        Ok(x)
      } else {
        Err("Inner error".to_string())
      };

      let nested_result: Result<Result<i64, String>, TestError> = if is_outer_ok {
        Ok(inner_result)
      } else {
        Err(TestError::Error1)
      };

      // Map the outer Result - using saturating multiplication to avoid overflow
      let double = |res: &Result<i64, String>| -> Result<i64, String> {
        match res {
          Ok(x) => Ok(x.saturating_mul(2)),
          Err(s) => Err(s.clone()),
        }
      };

      let result = Functor::map(nested_result, double);

      // Expected result
      let expected = if is_outer_ok {
        Ok(if is_inner_ok {
          Ok(x.saturating_mul(2))
        } else {
          Err("Inner error".to_string())
        })
      } else {
        Err(TestError::Error1)
      };

      prop_assert_eq!(result, expected);
    }
  }
}
