//! Traversable trait and implementations.
//!
//! A traversable is a type that can be traversed from left to right, performing an effect at each element.
//! It combines the capabilities of Functor and Foldable.

use crate::applicative::Applicative;
use crate::foldable::Foldable;
use crate::functor::Functor;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::{Mutex, RwLock};

/// A traversable is a type that can be traversed from left to right, performing an effect at each element.
///
/// # Type Parameters
/// * `T` - The type of elements in the structure
///
/// # Safety
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement Send + Sync + 'static
/// - The effect type F must implement Applicative
/// - All functions must be Send + Sync + 'static
pub trait Traversable<T: Send + Sync + 'static>: Functor<T> + Foldable<T> {
  /// Map each element of a structure to an action, evaluate these actions from left to right,
  /// and collect the results.
  fn traverse<F, U, G>(self, f: G) -> F::ApplicativeSelf<Self::HigherSelf<U>>
  where
    F: Applicative<U>,
    U: Send + Sync + 'static,
    G: FnMut(T) -> F::ApplicativeSelf<U> + Send + Sync + 'static,
    Self::HigherSelf<U>: Send + Sync + 'static;

  /// Evaluate each action in the structure from left to right, and collect the results.
  fn sequence<F, U>(self) -> F::ApplicativeSelf<Self::HigherSelf<U>>
  where
    F: Applicative<U>,
    T: Into<F::ApplicativeSelf<U>>,
    U: Send + Sync + 'static,
    Self::HigherSelf<U>: Send + Sync + 'static;
}

// Implementation for Option
impl<T: Send + Sync + 'static> Traversable<T> for Option<T> {
  fn traverse<F, U, G>(self, mut f: G) -> F::ApplicativeSelf<Option<U>>
  where
    F: Applicative<U>,
    U: Send + Sync + 'static,
    G: FnMut(T) -> F::ApplicativeSelf<U> + Send + Sync + 'static,
  {
    match self {
      Some(x) => F::map(f(x), Some),
      None => F::pure(None),
    }
  }

  fn sequence<F, U>(self) -> F::ApplicativeSelf<Option<U>>
  where
    F: Applicative<U>,
    T: Into<F::ApplicativeSelf<U>>,
    U: Send + Sync + 'static,
  {
    match self {
      Some(x) => F::map(x.into(), Some),
      None => F::pure(None),
    }
  }
}

// Implementation for Vec
impl<T: Send + Sync + 'static> Traversable<T> for Vec<T> {
  fn traverse<F, U, G>(self, mut f: G) -> F::ApplicativeSelf<Vec<U>>
  where
    F: Applicative<U>,
    U: Send + Sync + 'static,
    G: FnMut(T) -> F::ApplicativeSelf<U> + Send + Sync + 'static,
  {
    let mut result = F::pure(Vec::new());
    for x in self {
      result = F::ap(
        F::map(result, |mut v: Vec<U>| {
          move |u: U| {
            v.push(u);
            v
          }
        }),
        f(x),
      );
    }
    result
  }

  fn sequence<F, U>(self) -> F::ApplicativeSelf<Vec<U>>
  where
    F: Applicative<U>,
    T: Into<F::ApplicativeSelf<U>>,
    U: Send + Sync + 'static,
  {
    let mut result = F::pure(Vec::new());
    for x in self {
      result = F::ap(
        F::map(result, |mut v: Vec<U>| {
          move |u: U| {
            v.push(u);
            v
          }
        }),
        x.into(),
      );
    }
    result
  }
}

// Implementation for Result
impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Traversable<T> for Result<T, E> {
  fn traverse<F, U, G>(self, mut f: G) -> F::ApplicativeSelf<Result<U, E>>
  where
    F: Applicative<U>,
    U: Send + Sync + 'static,
    G: FnMut(T) -> F::ApplicativeSelf<U> + Send + Sync + 'static,
  {
    match self {
      Ok(x) => F::map(f(x), Ok),
      Err(e) => F::pure(Err(e)),
    }
  }

  fn sequence<F, U>(self) -> F::ApplicativeSelf<Result<U, E>>
  where
    F: Applicative<U>,
    T: Into<F::ApplicativeSelf<U>>,
    U: Send + Sync + 'static,
  {
    match self {
      Ok(x) => F::map(x.into(), Ok),
      Err(e) => F::pure(Err(e)),
    }
  }
}

// Implementation for Mutex
impl<T: Send + Sync + 'static> Traversable<T> for Mutex<T> {
  fn traverse<F, U, G>(self, mut f: G) -> F::ApplicativeSelf<Mutex<U>>
  where
    F: Applicative<U>,
    U: Send + Sync + 'static,
    G: FnMut(T) -> F::ApplicativeSelf<U> + Send + Sync + 'static,
  {
    F::map(f(self.into_inner().unwrap()), Mutex::new)
  }

  fn sequence<F, U>(self) -> F::ApplicativeSelf<Mutex<U>>
  where
    F: Applicative<U>,
    T: Into<F::ApplicativeSelf<U>>,
    U: Send + Sync + 'static,
  {
    F::map(self.into_inner().unwrap().into(), Mutex::new)
  }
}

// Implementation for RwLock
impl<T: Send + Sync + 'static> Traversable<T> for RwLock<T> {
  fn traverse<F, U, G>(self, mut f: G) -> F::ApplicativeSelf<RwLock<U>>
  where
    F: Applicative<U>,
    U: Send + Sync + 'static,
    G: FnMut(T) -> F::ApplicativeSelf<U> + Send + Sync + 'static,
  {
    F::map(f(self.into_inner().unwrap()), RwLock::new)
  }

  fn sequence<F, U>(self) -> F::ApplicativeSelf<RwLock<U>>
  where
    F: Applicative<U>,
    T: Into<F::ApplicativeSelf<U>>,
    U: Send + Sync + 'static,
  {
    F::map(self.into_inner().unwrap().into(), RwLock::new)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::category::Category;
  use proptest::prelude::*;
  use std::fmt::Debug;
  use std::sync::Arc;

  // Test applicative for Option
  struct OptionApplicative<T: Send + Sync + 'static>(PhantomData<T>);

  impl<T: Send + Sync + 'static> Category<T, T> for OptionApplicative<T> {
    type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> =
      Option<Arc<dyn Fn(A) -> B + Send + Sync>>;

    fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
      Some(Arc::new(|x| x))
    }

    fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
      f: Self::Morphism<A, B>,
      g: Self::Morphism<B, C>,
    ) -> Self::Morphism<A, C> {
      match (f, g) {
        (Some(f), Some(g)) => Some(Arc::new(move |x| g(f(x)))),
        _ => None,
      }
    }

    fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
    where
      F: Fn(A) -> B + Send + Sync + 'static,
    {
      Some(Arc::new(f))
    }

    fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
      f: Self::Morphism<A, B>,
    ) -> Self::Morphism<(A, C), (B, C)> {
      match f {
        Some(f) => Some(Arc::new(move |(a, c)| (f(a), c))),
        None => None,
      }
    }

    fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
      f: Self::Morphism<A, B>,
    ) -> Self::Morphism<(C, A), (C, B)> {
      match f {
        Some(f) => Some(Arc::new(move |(c, a)| (c, f(a)))),
        None => None,
      }
    }
  }

  impl<T: Send + Sync + 'static> Functor<T> for OptionApplicative<T> {
    type HigherSelf<U: Send + Sync + 'static> = Option<U>;

    fn map<U, F>(self, f: F) -> Self::HigherSelf<U>
    where
      F: FnMut(T) -> U + Send + Sync + 'static,
      U: Send + Sync + 'static,
    {
      match self {
        Some(x) => Some(f(x)),
        None => None,
      }
    }
  }

  impl<T: Send + Sync + 'static> Applicative<T> for OptionApplicative<T> {
    type ApplicativeSelf<U: Send + Sync + 'static> = Option<U>;

    fn pure<U>(value: U) -> Self::ApplicativeSelf<U>
    where
      U: Send + Sync + 'static,
    {
      Some(value)
    }

    fn ap<U, F>(self, f: Self::ApplicativeSelf<F>) -> Self::ApplicativeSelf<U>
    where
      F: FnMut(T) -> U + Send + Sync + 'static,
      U: Send + Sync + 'static,
    {
      match (self, f) {
        (Some(a), Some(mut f)) => Some(f(a)),
        _ => None,
      }
    }
  }

  // Test functions that implement Debug
  // These functions are chosen to cover various cases:
  // - Basic arithmetic operations
  // - Conditional operations
  // - Edge cases (zero, negative numbers)
  // - Identity-like operations
  const FUNCTIONS: &[fn(i32) -> Option<i32>] = &[
    |x| Some(x + 1),                               // Increment
    |x| Some(x * 2),                               // Double
    |x| Some(x - 1),                               // Decrement
    |x| Some(x / 2),                               // Halve
    |x| Some(x * x),                               // Square
    |x| Some(-x),                                  // Negate
    |x| if x > 0 { Some(x) } else { None },        // Positive only
    |x| if x % 2 == 0 { Some(x) } else { None },   // Even only
    |x| Some(x.abs()),                             // Absolute value
    |x| if x == 0 { None } else { Some(100 / x) }, // Invert with edge case
  ];

  proptest! {
      #[test]
      fn test_identity_laws(
          x in any::<i32>()
      ) {
          // Test Option
          let opt = Some(x);
          let result = opt.traverse::<OptionApplicative<_>, _, _>(|x| Some(x));
          assert_eq!(result, Some(Some(x)));

          // Test Vec
          let vec = vec![x];
          let result = vec.traverse::<OptionApplicative<_>, _, _>(|x| Some(x));
          assert_eq!(result, Some(vec![x]));

          // Test Result
          let res: Result<i32, &str> = Ok(x);
          let result = res.traverse::<OptionApplicative<_>, _, _>(|x| Some(x));
          assert_eq!(result, Some(Ok(x)));

          // Test Mutex
          let mutex = Mutex::new(x);
          let result = mutex.traverse::<OptionApplicative<_>, _, _>(|x| Some(x));
          assert_eq!(result.map(|m| m.into_inner().unwrap()), Some(x));

          // Test RwLock
          let rwlock = RwLock::new(x);
          let result = rwlock.traverse::<OptionApplicative<_>, _, _>(|x| Some(x));
          assert_eq!(result.map(|rw| rw.into_inner().unwrap()), Some(x));
      }

      #[test]
      fn test_composition_laws(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];

          // Test Option
          let opt = Some(x);
          let result1 = opt.traverse::<OptionApplicative<_>, _, _>(|x| f(x).and_then(g));
          let result2 = opt.traverse::<OptionApplicative<_>, _, _>(f).and_then(|opt| opt.traverse::<OptionApplicative<_>, _, _>(g));
          assert_eq!(result1, result2);

          // Test Vec
          let vec = vec![x];
          let result1 = vec.traverse::<OptionApplicative<_>, _, _>(|x| f(x).and_then(g));
          let result2 = vec.traverse::<OptionApplicative<_>, _, _>(f).and_then(|vec| vec.traverse::<OptionApplicative<_>, _, _>(g));
          assert_eq!(result1, result2);

          // Test Result
          let res: Result<i32, &str> = Ok(x);
          let result1 = res.traverse::<OptionApplicative<_>, _, _>(|x| f(x).and_then(g));
          let result2 = res.traverse::<OptionApplicative<_>, _, _>(f).and_then(|res| res.traverse::<OptionApplicative<_>, _, _>(g));
          assert_eq!(result1, result2);
      }

      #[test]
      fn test_edge_cases(
          x in any::<i32>()
      ) {
          // Test Option with None
          let opt: Option<i32> = None;
          let result = opt.traverse::<OptionApplicative<_>, _, _>(|x| Some(x));
          assert_eq!(result, Some(None));

          // Test Vec with empty
          let vec: Vec<i32> = vec![];
          let result = vec.traverse::<OptionApplicative<_>, _, _>(|x| Some(x));
          assert_eq!(result, Some(vec![]));

          // Test Result with Err
          let res: Result<i32, &str> = Err("error");
          let result = res.traverse::<OptionApplicative<_>, _, _>(|x| Some(x));
          assert_eq!(result, Some(Err("error")));

          // Test with functions that return None
          let f = |x| if x > 0 { Some(x) } else { None };
          let opt = Some(x);
          let result = opt.traverse::<OptionApplicative<_>, _, _>(f);
          let expected = if x > 0 { Some(Some(x)) } else { None };
          assert_eq!(result, expected);
      }

      #[test]
      fn test_sequence_properties(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];

          // Test Option
          let opt: Option<Option<i32>> = Some(f(x));
          let result = opt.sequence::<OptionApplicative<_>, _>();
          assert_eq!(result, f(x).map(Some));

          // Test Vec
          let vec: Vec<Option<i32>> = vec![f(x), f(x + 1)];
          let result = vec.sequence::<OptionApplicative<_>, _>();
          let expected = if vec.iter().all(|x| x.is_some()) {
              Some(vec.into_iter().map(|x| x.unwrap()).collect())
          } else {
              None
          };
          assert_eq!(result, expected);

          // Test Result
          let res: Result<Option<i32>, &str> = Ok(f(x));
          let result = res.sequence::<OptionApplicative<_>, _>();
          assert_eq!(result, f(x).map(Ok));

          // Test Mutex
          let mutex = Mutex::new(f(x));
          let result = mutex.sequence::<OptionApplicative<_>, _>();
          assert_eq!(result.map(|m| m.into_inner().unwrap()), f(x));

          // Test RwLock
          let rwlock = RwLock::new(f(x));
          let result = rwlock.sequence::<OptionApplicative<_>, _>();
          assert_eq!(result.map(|rw| rw.into_inner().unwrap()), f(x));
      }
  }
}
