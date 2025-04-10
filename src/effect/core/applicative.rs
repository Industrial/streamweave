//! Applicative trait and implementations.
//!
//! An applicative functor is a functor that can apply functions within the functor context.

use super::functor::Functor;

/// The Applicative trait defines operations for applicative functors.
pub trait Applicative<A>: Functor<A> {
  /// Lifts a value into the applicative context.
  fn pure(a: A) -> Self;

  /// Applies a function within the applicative context.
  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B;
}

// Implementation for Option
impl<A> Applicative<A> for Option<A> {
  fn pure(a: A) -> Self {
    Some(a)
  }

  fn ap<B, F>(self, f: Option<F>) -> Option<B>
  where
    F: FnMut(A) -> B,
  {
    match (self, f) {
      (Some(a), Some(mut f)) => Some(f(a)),
      _ => None,
    }
  }
}

// Implementation for Vec
impl<A> Applicative<A> for Vec<A> {
  fn pure(a: A) -> Self {
    vec![a]
  }

  fn ap<B, F>(self, fs: Vec<F>) -> Vec<B>
  where
    F: FnMut(A) -> B,
  {
    fs.into_iter()
      .flat_map(|mut f| self.iter().cloned().map(move |a| f(a)))
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
