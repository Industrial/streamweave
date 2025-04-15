use crate::applicative::Applicative;
use proptest::prelude::*;
use std::clone::Clone;
use std::fmt::Debug;
use std::marker::Send;
use std::marker::Sync;
use std::ops::Deref;

/// Newtype wrapper for Option
#[derive(Clone, Debug)]
pub struct OptAlt<T>(Option<T>);

/// Newtype wrapper for Result
#[derive(Clone, Debug)]
pub struct ResultAlt<T, E>(Result<T, E>);

/// Newtype wrapper for Vec
#[derive(Clone, Debug)]
pub struct VecAlt<T>(Vec<T>);

impl<T: Send + Sync + 'static> Applicative<T> for OptAlt<T> {
  type HigherSelf<U: Send + Sync + 'static> = OptAlt<U>;

  fn pure(value: T) -> Self::HigherSelf<T> {
    OptAlt(Some(value))
  }

  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    OptAlt(self.0.and_then(|a| f.0.map(|mut f| f(a))))
  }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Applicative<T> for ResultAlt<T, E> {
  type HigherSelf<U: Send + Sync + 'static> = ResultAlt<U, E>;

  fn pure(value: T) -> Self::HigherSelf<T> {
    ResultAlt(Ok(value))
  }

  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    ResultAlt(match (self.0, f.0) {
      (Ok(a), Ok(mut f)) => Ok(f(a)),
      (Err(e), _) => Err(e),
      (_, Err(e)) => Err(e),
    })
  }
}

impl<T: Send + Sync + Clone + 'static> Applicative<T> for VecAlt<T> {
  type HigherSelf<U: Send + Sync + 'static> = VecAlt<U>;

  fn pure(value: T) -> Self::HigherSelf<T> {
    VecAlt(vec![value])
  }

  fn ap<B, F>(self, fs: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    VecAlt(
      fs.0
        .into_iter()
        .flat_map(|mut f| self.0.iter().cloned().map(move |a| f(a)))
        .collect(),
    )
  }
}

/// The Alternative trait represents types that are both Applicative and Monoid.
/// It provides operations for choice and repetition.
pub trait Alternative<T>: Applicative<T> + Clone {
  /// The empty value for the Alternative type.
  fn empty() -> Self;

  /// Combines two Alternative values, choosing the first non-empty one.
  fn alt(self, other: Self) -> Self;

  /// Returns Some if the given predicate holds, otherwise returns empty.
  fn some<F>(self, f: F) -> Self
  where
    F: Fn(&T) -> bool;

  /// Returns the value if it's non-empty, otherwise returns empty.
  fn many(self) -> Self {
    self.alt(Self::empty())
  }

  /// Helper method to unwrap the value
  fn unwrap(&self) -> &T;
}

impl<T: Send + Sync + Clone + 'static> Alternative<T> for OptAlt<T> {
  fn empty() -> Self {
    OptAlt(None)
  }

  fn alt(self, other: Self) -> Self {
    OptAlt(self.0.or(other.0))
  }

  fn some<F>(self, f: F) -> Self
  where
    F: Fn(&T) -> bool,
  {
    match &self.0 {
      Some(value) if f(value) => self,
      _ => Self::empty(),
    }
  }

  fn unwrap(&self) -> &T {
    self.0.as_ref().unwrap()
  }
}

impl<T: Send + Sync + Clone + 'static, E: Default + Send + Sync + Debug + Clone + 'static>
  Alternative<T> for ResultAlt<T, E>
{
  fn empty() -> Self {
    ResultAlt(Err(E::default()))
  }

  fn alt(self, other: Self) -> Self {
    ResultAlt(self.0.or(other.0))
  }

  fn some<F>(self, f: F) -> Self
  where
    F: Fn(&T) -> bool,
  {
    match &self.0 {
      Ok(value) if f(value) => self,
      _ => Self::empty(),
    }
  }

  fn unwrap(&self) -> &T {
    self.0.as_ref().unwrap()
  }
}

impl<T: Send + Sync + Clone + 'static> Alternative<T> for VecAlt<T> {
  fn empty() -> Self {
    VecAlt(Vec::new())
  }

  fn alt(self, other: Self) -> Self {
    if self.0.is_empty() {
      other
    } else {
      self
    }
  }

  fn some<F>(self, f: F) -> Self
  where
    F: Fn(&T) -> bool,
  {
    if self.0.iter().any(f) {
      self
    } else {
      Self::empty()
    }
  }

  fn unwrap(&self) -> &T {
    &self.0[0]
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  proptest! {
      #[test]
      fn test_option_alternative_empty(opt: Option<i32>) {
          let empty = OptAlt::<i32>::empty();
          assert_eq!(empty.0, None);
          assert_eq!(OptAlt(opt.clone()).alt(empty).0, opt);
      }

      #[test]
      fn test_option_alternative_alt(opt1: Option<i32>, opt2: Option<i32>) {
          let result = OptAlt(opt1.clone()).alt(OptAlt(opt2.clone()));
          assert_eq!(result.0, opt1.or(opt2));
      }

      #[test]
      fn test_option_alternative_some(opt: Option<i32>, value: i32) {
          let result = OptAlt(opt.clone()).some(|x| *x == value);
          assert_eq!(result.0, if opt == Some(value) { opt } else { None });
      }

      #[test]
      fn test_option_alternative_many(opt: Option<i32>) {
          let result = OptAlt(opt.clone()).many();
          assert_eq!(result.0, opt.or(None));
      }

      #[test]
      fn test_result_alternative_empty(result: Result<i32, i32>) {
          let empty = ResultAlt::<i32, i32>::empty();
          assert_eq!(empty.0, Err(0));
          assert_eq!(ResultAlt(result.clone()).alt(empty).0, result);
      }

      #[test]
      fn test_result_alternative_alt(result1: Result<i32, i32>, result2: Result<i32, i32>) {
          let result = ResultAlt(result1.clone()).alt(ResultAlt(result2.clone()));
          assert_eq!(result.0, result1.or(result2));
      }

      #[test]
      fn test_vec_alternative_empty(vec: Vec<i32>) {
          let empty = VecAlt::<i32>::empty();
          assert!(empty.0.is_empty());
          assert_eq!(VecAlt(vec.clone()).alt(empty).0, vec);
      }

      #[test]
      fn test_vec_alternative_alt(vec1: Vec<i32>, vec2: Vec<i32>) {
          let result = VecAlt(vec1.clone()).alt(VecAlt(vec2.clone()));
          assert_eq!(result.0, if vec1.is_empty() { vec2 } else { vec1 });
      }
  }
}
