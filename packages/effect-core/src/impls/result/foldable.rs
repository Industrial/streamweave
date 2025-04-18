use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe, E: CloneableThreadSafe> Foldable<T> for Result<T, E> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    match self {
      Ok(x) => f(init, &x),
      Err(_) => init,
    }
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    match self {
      Ok(x) => f(&x, init),
      Err(_) => init,
    }
  }

  fn reduce<F>(self, _: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For Result, reduce just extracts the Ok value
    self.ok()
  }

  fn reduce_right<F>(self, _: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // For Result, reduce_right also just extracts the Ok value
    self.ok()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::fmt::Debug;

  // Define custom error type for testing
  #[derive(Debug, PartialEq, Eq, Clone)]
  struct TestError(i32);

  impl CloneableThreadSafe for TestError {}

  // Define test functions with overflow protection for property-based testing
  const INT_FUNCTIONS: &[fn(i32, &i32) -> i32] = &[
    |acc, x| acc.saturating_add(*x),
    |acc, x| acc.saturating_mul(*x),
    |acc, x| acc.saturating_sub(*x),
    |acc, x| if *x != 0 { acc / x } else { acc },
  ];

  const INT_FUNCTIONS_RIGHT: &[fn(&i32, i32) -> i32] = &[
    |x, acc| acc.saturating_add(*x),
    |x, acc| acc.saturating_mul(*x),
    |x, acc| x.saturating_sub(acc),
    |x, acc| if acc != 0 { *x / acc } else { *x },
  ];

  // Helper to create test Results
  fn ok_result<T: Clone, E: Clone>(value: T) -> Result<T, E> {
    Ok(value)
  }

  fn err_result<T, E: Clone>(error: E) -> Result<T, E> {
    Err(error)
  }

  // Test fold with Ok value
  #[test]
  fn test_fold_ok() {
    let ok: Result<i32, TestError> = Ok(5);
    let result = ok.fold(10, |acc, x| acc + x);
    assert_eq!(result, 15);
  }

  // Test fold with Err value
  #[test]
  fn test_fold_err() {
    let err: Result<i32, TestError> = Err(TestError(42));
    let result = err.fold(10, |acc, x| acc + x);
    assert_eq!(result, 10, "fold should return the initial value for Err");
  }

  // Property-based test for fold
  proptest! {
    #[test]
    fn test_fold_properties(
      x in -1000..1000i32,
      init in -10..10i32,
      is_ok in proptest::bool::ANY,
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];

      let result = if is_ok {
        let ok: Result<i32, TestError> = Ok(x);
        ok.fold(init, f)
      } else {
        let err: Result<i32, TestError> = Err(TestError(x));
        err.fold(init, f)
      };

      let expected = if is_ok {
        f(init, &x)
      } else {
        init
      };

      assert_eq!(result, expected);
    }
  }

  // Test fold_right with Ok value
  #[test]
  fn test_fold_right_ok() {
    let ok: Result<i32, TestError> = Ok(5);
    let result = ok.fold_right(10, |x, acc| x + acc);
    assert_eq!(result, 15);
  }

  // Test fold_right with Err value
  #[test]
  fn test_fold_right_err() {
    let err: Result<i32, TestError> = Err(TestError(42));
    let result = err.fold_right(10, |x, acc| x + acc);
    assert_eq!(
      result, 10,
      "fold_right should return the initial value for Err"
    );
  }

  // Property-based test for fold_right
  proptest! {
    #[test]
    fn test_fold_right_properties(
      x in -1000..1000i32,
      init in -10..10i32,
      is_ok in proptest::bool::ANY,
      f_idx in 0..INT_FUNCTIONS_RIGHT.len()
    ) {
      let f = INT_FUNCTIONS_RIGHT[f_idx];

      let result = if is_ok {
        let ok: Result<i32, TestError> = Ok(x);
        ok.fold_right(init, f)
      } else {
        let err: Result<i32, TestError> = Err(TestError(x));
        err.fold_right(init, f)
      };

      let expected = if is_ok {
        f(&x, init)
      } else {
        init
      };

      assert_eq!(result, expected);
    }
  }

  // Test reduce with Ok value
  #[test]
  fn test_reduce_ok() {
    let ok: Result<i32, TestError> = Ok(42);
    let result = ok.reduce(|_, _| unreachable!());
    assert_eq!(
      result,
      Some(42),
      "reduce should return the value wrapped in Some for Ok"
    );
  }

  // Test reduce with Err value
  #[test]
  fn test_reduce_err() {
    let err: Result<i32, TestError> = Err(TestError(42));
    let result = err.reduce(|_, _| unreachable!());
    assert_eq!(result, None, "reduce should return None for Err");
  }

  // Property-based test for reduce
  proptest! {
    #[test]
    fn test_reduce_properties(
      x in -1000..1000i32,
      is_ok in proptest::bool::ANY
    ) {
      let result = if is_ok {
        let ok: Result<i32, TestError> = Ok(x);
        ok.reduce(|_, _| unreachable!())
      } else {
        let err: Result<i32, TestError> = Err(TestError(x));
        err.reduce(|_, _| unreachable!())
      };

      let expected = if is_ok {
        Some(x)
      } else {
        None
      };

      assert_eq!(result, expected);
    }
  }

  // Test reduce_right with Ok value
  #[test]
  fn test_reduce_right_ok() {
    let ok: Result<i32, TestError> = Ok(42);
    let result = ok.reduce_right(|_, _| unreachable!());
    assert_eq!(
      result,
      Some(42),
      "reduce_right should return the value wrapped in Some for Ok"
    );
  }

  // Test reduce_right with Err value
  #[test]
  fn test_reduce_right_err() {
    let err: Result<i32, TestError> = Err(TestError(42));
    let result = err.reduce_right(|_, _| unreachable!());
    assert_eq!(result, None, "reduce_right should return None for Err");
  }

  // Property-based test for reduce_right
  proptest! {
    #[test]
    fn test_reduce_right_properties(
      x in -1000..1000i32,
      is_ok in proptest::bool::ANY
    ) {
      let result = if is_ok {
        let ok: Result<i32, TestError> = Ok(x);
        ok.reduce_right(|_, _| unreachable!())
      } else {
        let err: Result<i32, TestError> = Err(TestError(x));
        err.reduce_right(|_, _| unreachable!())
      };

      let expected = if is_ok {
        Some(x)
      } else {
        None
      };

      assert_eq!(result, expected);
    }
  }

  // Test identity law: fold(f, init) of identity function should return init for Err
  #[test]
  fn test_fold_identity_law_err() {
    let err: Result<i32, TestError> = Err(TestError(42));
    let init = 10;
    let result = err.fold(init, |_, _| unreachable!());
    assert_eq!(result, init);
  }

  // Test with different types
  #[test]
  fn test_foldable_different_types() {
    // Result with string value
    let ok: Result<&str, TestError> = Ok("hello");
    let string_length = ok.fold(0, |acc, s| acc + s.len());
    assert_eq!(string_length, 5);

    // Result with string value, used as an accumulator
    let ok: Result<i32, TestError> = Ok(5);
    let as_string = ok.fold("Value: ".to_string(), |mut acc, x| {
      acc.push_str(&x.to_string());
      acc
    });
    assert_eq!(as_string, "Value: 5");

    // Result with collection
    let ok: Result<Vec<i32>, TestError> = Ok(vec![1, 2, 3]);
    let sum = ok.fold(0, |acc, v| acc + v.iter().sum::<i32>());
    assert_eq!(sum, 6);
  }

  // Test with stateful function
  #[test]
  fn test_fold_with_stateful_function() {
    let ok: Result<i32, TestError> = Ok(5);
    let mut call_count = 0;

    let result = ok.fold(10, |acc, x| {
      call_count += 1;
      acc + x
    });

    assert_eq!(result, 15);
    assert_eq!(call_count, 1, "Function should be called once for Ok");

    let err: Result<i32, TestError> = Err(TestError(5));
    call_count = 0;

    let result = err.fold(10, |acc, x| {
      call_count += 1;
      acc + x
    });

    assert_eq!(result, 10);
    assert_eq!(call_count, 0, "Function should not be called for Err");
  }

  // Test with complex values and errors
  #[test]
  fn test_fold_with_complex_types() {
    // Result with nested Result as its value
    let nested_ok: Result<Result<i32, &str>, TestError> = Ok(Ok(42));
    let result = nested_ok.fold(0, |acc, inner| match inner {
      Ok(x) => acc + x,
      Err(_) => acc,
    });
    assert_eq!(result, 42);

    // Fold that returns a Result
    let ok: Result<i32, TestError> = Ok(5);
    let result: Result<i32, &str> = ok.fold(Ok(10), |acc, x| match acc {
      Ok(v) => Ok(v + x),
      Err(e) => Err(e),
    });
    assert_eq!(result, Ok(15));
  }
}
