//! Applicative trait and implementations.
//!
//! An applicative functor is a functor that can apply functions within the functor context.

/// The Applicative trait represents a type that can apply functions within a context.
pub trait Applicative<T> {
  /// The higher-kinded type that results from applying a function
  type HigherSelf<U: Send + Sync + 'static>;

  /// Lifts a value into the applicative context
  fn pure(value: T) -> Self::HigherSelf<T>
  where
    T: Send + Sync + 'static;

  /// Applies a function within the applicative context
  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static;
}

/// A trait for types that can apply functions within a context
pub trait Applicable<T>: Applicative<T> {}

// Implement Applicable for all types that implement Applicative
impl<T, A> Applicable<T> for A
where
  A: Applicative<T>,
  T: Send + Sync + 'static,
{
}

// Implementation for Option
impl<A: Send + Sync + 'static> Applicative<A> for Option<A> {
  type HigherSelf<U: Send + Sync + 'static> = Option<U>;

  fn pure(a: A) -> Self::HigherSelf<A> {
    Some(a)
  }

  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    match (self, f) {
      (Some(a), Some(mut f)) => Some(f(a)),
      _ => None,
    }
  }
}

// Implementation for Vec
impl<A: Send + Sync + 'static + Clone> Applicative<A> for Vec<A> {
  type HigherSelf<U: Send + Sync + 'static> = Vec<U>;

  fn pure(a: A) -> Self::HigherSelf<A> {
    vec![a]
  }

  fn ap<B, F>(self, fs: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    fs.into_iter()
      .flat_map(|f| self.iter().cloned().map(f))
      .collect()
  }
}

// Implementation for Result
impl<A: Send + Sync + 'static, E: Send + Sync + 'static> Applicative<A> for Result<A, E> {
  type HigherSelf<U: Send + Sync + 'static> = Result<U, E>;

  fn pure(a: A) -> Self::HigherSelf<A> {
    Ok(a)
  }

  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    match (self, f) {
      (Ok(a), Ok(mut f)) => Ok(f(a)),
      (Err(e), _) => Err(e),
      (_, Err(e)) => Err(e),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Define a simple applicative for testing
  #[derive(Debug, PartialEq, Clone)]
  struct TestApplicative<T>(T);

  impl<T> TestApplicative<T> {
    fn new(value: T) -> Self {
      TestApplicative(value)
    }
  }

  impl<T: Send + Sync + 'static> Applicative<T> for TestApplicative<T> {
    type HigherSelf<U: Send + Sync + 'static> = TestApplicative<U>;

    fn pure(value: T) -> Self::HigherSelf<T> {
      TestApplicative(value)
    }

    fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
    where
      F: FnMut(T) -> B + Send + Sync + 'static,
      B: Send + Sync + 'static,
    {
      let TestApplicative(mut func) = f;
      let TestApplicative(value) = self;
      TestApplicative(func(value))
    }
  }

  // Test applicative laws
  #[test]
  fn test_applicative_laws() {
    // Identity law: pure id <*> v = v
    let v = TestApplicative::new(42);
    let id = TestApplicative::new(|x: i32| x);
    assert_eq!(v.clone().ap(id), v);

    // Homomorphism law: pure f <*> pure x = pure (f x)
    let f = |x: i32| x * 2;
    let x = TestApplicative::new(42);
    let lhs = x.clone().ap(TestApplicative::new(f));
    let rhs = TestApplicative::new(f(42));
    assert_eq!(lhs, rhs);

    // Interchange law: u <*> pure y = pure ($ y) <*> u
    let u = TestApplicative::new(|x: i32| x * 2);
    let y = 42;
    let lhs = TestApplicative::new(y).ap(u.clone());
    let rhs = TestApplicative::new(84);
    assert_eq!(lhs, rhs);
  }

  // Test basic operations
  #[test]
  fn test_pure() {
    let a: TestApplicative<i32> = TestApplicative::pure(42);
    assert_eq!(a, TestApplicative(42));
  }

  #[test]
  fn test_ap() {
    let a = TestApplicative::new(42);
    let f = TestApplicative::pure(|x: i32| x * 2);
    let result = a.ap(f);
    assert_eq!(result, TestApplicative(84));
  }

  // Test type conversions
  #[test]
  fn test_type_conversions() {
    let a = TestApplicative::new(42);
    let f = TestApplicative::pure(|x: i32| x.to_string());
    let result = a.ap(f);
    assert_eq!(result, TestApplicative("42".to_string()));
  }

  // Test composition
  #[test]
  fn test_composition() {
    let a = TestApplicative::new(42);
    let f1 = TestApplicative::pure(|x: i32| x * 2);
    let f2 = TestApplicative::pure(|x: i32| x + 1);
    let result = a.ap(f1).ap(f2);
    assert_eq!(result, TestApplicative(85));
  }

  mod option_tests {
    use super::*;

    #[test]
    fn test_some_with_some_fn() {
      let some = Some(42);
      let some_fn = Some(|x: i32| x * 2);
      let result = some.ap(some_fn);
      assert_eq!(result, Some(84));
    }

    #[test]
    fn test_none_with_some_fn() {
      let none: Option<i32> = None;
      let some_fn = Some(|x: i32| x * 2);
      let result = none.ap(some_fn);
      assert_eq!(result, None);
    }

    #[test]
    fn test_some_with_none_fn() {
      let some = Some(42);
      let none_fn: Option<fn(i32) -> i32> = None;
      let result = some.ap(none_fn);
      assert_eq!(result, None);
    }

    #[test]
    fn test_type_conversion() {
      let some = Some(42);
      let some_fn = Some(|x: i32| x.to_string());
      let result = some.ap(some_fn);
      assert_eq!(result, Some("42".to_string()));
    }

    #[test]
    fn test_composition() {
      let some = Some(42);
      let f1 = Some(|x: i32| x * 2);
      let f2 = Some(|x: i32| x + 1);
      let result = some.ap(f1).ap(f2);
      assert_eq!(result, Some(85));
    }
  }

  mod vec_tests {
    use super::*;

    #[test]
    fn test_empty_vec() {
      let empty: Vec<i32> = vec![];
      let fns: Vec<fn(i32) -> i32> = vec![];
      let result = empty.ap(fns);
      let expected: Vec<i32> = vec![];
      assert_eq!(result, expected);
    }

    #[test]
    fn test_single_element() {
      let vec = vec![42];
      let fns = vec![|x: i32| x * 2];
      let result = vec.ap(fns);
      assert_eq!(result, vec![84]);
    }

    #[test]
    fn test_multiple_elements() {
      let vec = vec![1, 2, 3];
      let fns = vec![|x: i32| x * 2, |x: i32| x + 1];
      let result = vec.ap(fns);
      assert_eq!(result, vec![2, 4, 6, 2, 3, 4]);
    }

    #[test]
    fn test_type_conversion() {
      let vec = vec![1, 2, 3];
      let fns = vec![|x: i32| x.to_string()];
      let result = vec.ap(fns);
      assert_eq!(result, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_mutating_closure() {
      let mut counter = 0;
      let vec = vec![1, 2, 3];
      let fns = vec![move |x: i32| {
        counter += 1;
        x * counter
      }];
      let result = vec.ap(fns);
      assert_eq!(result, vec![1, 4, 9]);
    }

    #[test]
    fn test_ordering_preservation() {
      let vec = vec![3, 1, 4, 1, 5, 9];
      let fns = vec![|x: i32| x * 2];
      let result = vec.ap(fns);
      assert_eq!(result, vec![6, 2, 8, 2, 10, 18]);
    }
  }

  mod result_tests {
    use super::*;

    #[test]
    fn test_ok_with_ok_fn() {
      let ok: Result<i32, &str> = Ok(42);
      let ok_fn: Result<fn(i32) -> i32, &str> = Ok(|x: i32| x * 2);
      let result = ok.ap(ok_fn);
      assert_eq!(result, Ok(84));
    }

    #[test]
    fn test_err_with_ok_fn() {
      let err: Result<i32, &str> = Err("error");
      let ok_fn: Result<fn(i32) -> i32, &str> = Ok(|x: i32| x * 2);
      let result = err.ap(ok_fn);
      assert_eq!(result, Err("error"));
    }

    #[test]
    fn test_ok_with_err_fn() {
      let ok: Result<i32, &str> = Ok(42);
      let err_fn: Result<fn(i32) -> i32, &str> = Err("error");
      let result = ok.ap(err_fn);
      assert_eq!(result, Err("error"));
    }

    #[test]
    fn test_type_conversion() {
      let ok: Result<i32, &str> = Ok(42);
      let ok_fn: Result<fn(i32) -> String, &str> = Ok(|x: i32| x.to_string());
      let result = ok.ap(ok_fn);
      assert_eq!(result, Ok("42".to_string()));
    }

    #[test]
    fn test_composition() {
      let ok: Result<i32, &str> = Ok(42);
      let f1: Result<fn(i32) -> i32, &str> = Ok(|x: i32| x * 2);
      let f2: Result<fn(i32) -> i32, &str> = Ok(|x: i32| x + 1);
      let result = ok.ap(f1).ap(f2);
      assert_eq!(result, Ok(85));
    }
  }
}
