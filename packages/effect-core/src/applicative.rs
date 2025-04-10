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

#[cfg(test)]
mod tests {
  use super::*;

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
}
