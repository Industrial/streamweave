use crate::applicative::Applicative;
use crate::effect::Effect;
use std::clone::Clone;
use std::marker::Send;
use std::marker::Sync;

/// The Alternative trait represents types that are both Applicative and Monoid.
/// It provides operations for choice and repetition.
pub trait Alternative<T>: Applicative<T> {
  /// The type returned by the many operation
  type ManyOutput;

  /// The empty value for the Alternative type.
  fn empty() -> Self;

  /// Combines two Alternative values, choosing the first non-empty one.
  fn alt(self, other: Self) -> Self;

  /// Returns the value if it's non-empty, otherwise returns empty.
  fn many(self) -> Self::ManyOutput;
}

impl<T: Send + Sync + 'static> Alternative<T> for Option<T> {
  type ManyOutput = Self;

  fn empty() -> Self {
    None
  }

  fn alt(self, other: Self) -> Self {
    self.or(other)
  }

  fn many(self) -> Self::ManyOutput {
    self.alt(Self::empty())
  }
}

impl<T: Send + Sync + 'static, E: Default + Send + Sync + 'static> Alternative<T> for Result<T, E> {
  type ManyOutput = Self;

  fn empty() -> Self {
    Err(E::default())
  }

  fn alt(self, other: Self) -> Self {
    self.or(other)
  }

  fn many(self) -> Self::ManyOutput {
    self.alt(Self::empty())
  }
}

impl<T: Send + Sync + Clone + 'static> Alternative<T> for Vec<T> {
  type ManyOutput = Self;

  fn empty() -> Self {
    Vec::new()
  }

  fn alt(self, other: Self) -> Self {
    if self.is_empty() {
      other
    } else {
      self
    }
  }

  fn many(self) -> Self::ManyOutput {
    self.alt(Self::empty())
  }
}

impl<T: Send + Sync + 'static, E: Default + Send + Sync + 'static> Alternative<T> for Effect<T, E> {
  type ManyOutput = Self;

  fn empty() -> Self {
    Effect::new(Box::pin(async { Err(E::default()) }))
  }

  fn alt(self, other: Self) -> Self {
    Effect::new(Box::pin(async move {
      match self.await {
        Ok(value) => Ok(value),
        Err(_) => other.await,
      }
    }))
  }

  fn many(self) -> Self::ManyOutput {
    self.alt(Self::empty())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  fn result_strategy() -> impl Strategy<Value = Result<i32, i32>> {
    prop_oneof![Just(Err(0)), any::<i32>().prop_map(Ok)]
  }

  proptest! {
      #[test]
      fn test_option_alternative_empty(opt: Option<i32>) {
          let empty = Option::<i32>::empty();
          assert_eq!(empty, None);
          assert_eq!(opt.alt(empty), opt);
      }

      #[test]
      fn test_option_alternative_alt(opt1: Option<i32>, opt2: Option<i32>) {
          let result = opt1.alt(opt2);
          assert_eq!(result, opt1.or(opt2));
      }

      #[test]
      fn test_option_alternative_many(opt: Option<i32>) {
          let result = opt.many();
          assert_eq!(result, opt.or(None));
      }

      #[test]
      fn test_result_alternative_empty(result in result_strategy()) {
          let empty = Result::<i32, i32>::empty();
          assert_eq!(empty, Err(0));
          assert_eq!(result.alt(empty), result);
      }

      #[test]
      fn test_result_alternative_alt(result1: Result<i32, i32>, result2: Result<i32, i32>) {
          let result = result1.alt(result2);
          assert_eq!(result, result1.or(result2));
      }

      #[test]
      fn test_result_alternative_many(result: Result<i32, i32>) {
          let result = result.many();
          assert_eq!(result, result.or(Err(0)));
      }

      #[test]
      fn test_vec_alternative_empty(vec: Vec<i32>) {
          let empty = Vec::<i32>::empty();
          assert!(empty.is_empty());
          assert_eq!(vec.clone().alt(empty), vec);
      }

      #[test]
      fn test_vec_alternative_alt(vec1: Vec<i32>, vec2: Vec<i32>) {
          let result = vec1.clone().alt(vec2.clone());
          assert_eq!(result, if vec1.is_empty() { vec2 } else { vec1 });
      }

      #[test]
      fn test_vec_alternative_many(vec: Vec<i32>) {
          let result = vec.clone().many();
          assert_eq!(result, if vec.is_empty() { Vec::new() } else { vec });
      }
  }
}
