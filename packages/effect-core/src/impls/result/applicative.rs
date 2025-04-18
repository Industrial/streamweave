use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe, E: CloneableThreadSafe> Applicative<A> for Result<A, E> {
  fn pure<B>(value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe,
  {
    Ok(value)
  }

  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    match (self, f) {
      (Ok(a), Ok(mut f)) => Ok(f(&a)),
      (Err(e), _) => Err(e),
      (_, Err(e)) => Err(e),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // A simple error type for testing
  #[derive(Debug, PartialEq, Clone)]
  struct TestError;

  // Test functions with overflow protection
  const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  // Identity law: pure(id) <*> v = v
  #[test]
  fn test_identity_law() {
    let value: Result<i64, TestError> = Ok(42);
    let id_fn = Result::<i64, TestError>::pure(|x: &i64| *x);
    let result = value.clone().ap(id_fn);
    assert_eq!(result, value);

    // Test with Err
    let err: Result<i64, TestError> = Err(TestError);
    let id_fn = Result::<i64, TestError>::pure(|x: &i64| *x);
    let result = err.clone().ap(id_fn);
    assert_eq!(result, err);
  }

  // Homomorphism law: pure(f) <*> pure(x) = pure(f(x))
  #[test]
  fn test_homomorphism_law() {
    let x = 42i64;
    let f = |x: &i64| x.to_string();

    let left: Result<String, TestError> =
      Result::<i64, TestError>::pure(x).ap(Result::<i64, TestError>::pure(f));
    let right: Result<String, TestError> = Result::<String, TestError>::pure(f(&x));

    assert_eq!(left, right);
  }

  // Type conversion tests
  #[test]
  fn test_type_conversions() {
    let value: Result<i64, TestError> = Ok(42);
    let f: Result<fn(&i64) -> String, TestError> = Ok(|x: &i64| x.to_string());
    let result = value.ap(f);
    assert_eq!(result, Ok("42".to_string()));

    // Multiple type conversions
    let value: Result<&str, TestError> = Ok("hello");
    let f: Result<fn(&&str) -> usize, TestError> = Ok(|s: &&str| s.len());
    let result = value.ap(f);
    assert_eq!(result, Ok(5));

    // Struct creation
    #[derive(Debug, PartialEq, Clone)]
    struct Person {
      name: String,
      age: i64,
    }

    let name: Result<String, TestError> = Ok("Alice".to_string());
    let age: Result<i64, TestError> = Ok(30);

    // Use owned values instead of references
    let create_person = |name: String| move |age: i64| Person { name, age };

    // Use map and and_then instead of ap to avoid lifetime issues
    let person_fn = name.clone().map(create_person);
    let person = match (age.clone(), person_fn) {
      (Ok(a), Ok(f)) => Ok(f(a)),
      (Err(e), _) => Err(e),
      (_, Err(e)) => Err(e),
    };

    assert_eq!(
      person,
      Ok(Person {
        name: "Alice".to_string(),
        age: 30
      })
    );
  }

  proptest! {
    // Property-based test for identity law
    #[test]
    fn test_identity_law_prop(x in any::<i64>().prop_filter("Value too large", |v| *v < 10000)) {
      let value: Result<i64, TestError> = Ok(x);
      let id_fn = Result::<i64, TestError>::pure(|x: &i64| *x);
      let result = value.clone().ap(id_fn);
      prop_assert_eq!(result, value);
    }

    // Property-based test for homomorphism law
    #[test]
    fn test_homomorphism_law_prop(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];

      let left: Result<i64, TestError> = Result::<i64, TestError>::pure(x).ap(Result::<i64, TestError>::pure(f));
      let right: Result<i64, TestError> = Result::<i64, TestError>::pure(f(&x));

      prop_assert_eq!(left, right);
    }
  }
}
