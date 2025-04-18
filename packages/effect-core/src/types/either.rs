use crate::types::threadsafe::ThreadSafe;

/// A type representing a value of one of two possible types (a disjoint union).
///
/// This is similar to `Result<T, E>` but without the semantic meaning of success/error.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Either<L, R> {
  /// The left variant of the Either type
  Left(L),
  /// The right variant of the Either type
  Right(R),
}

impl<L, R> Either<L, R> {
  /// Creates a new `Either::Left` value.
  pub fn left(value: L) -> Self {
    Either::Left(value)
  }

  /// Creates a new `Either::Right` value.
  pub fn right(value: R) -> Self {
    Either::Right(value)
  }

  /// Returns true if this is a Left value.
  pub fn is_left(&self) -> bool {
    matches!(self, Either::Left(_))
  }

  /// Returns true if this is a Right value.
  pub fn is_right(&self) -> bool {
    matches!(self, Either::Right(_))
  }

  /// Maps the left value using the given function, leaving a right value untouched.
  pub fn map_left<U, F>(self, f: F) -> Either<U, R>
  where
    F: FnOnce(L) -> U,
  {
    match self {
      Either::Left(l) => Either::Left(f(l)),
      Either::Right(r) => Either::Right(r),
    }
  }

  /// Maps the right value using the given function, leaving a left value untouched.
  pub fn map_right<U, F>(self, f: F) -> Either<L, U>
  where
    F: FnOnce(R) -> U,
  {
    match self {
      Either::Left(l) => Either::Left(l),
      Either::Right(r) => Either::Right(f(r)),
    }
  }

  /// Applies one of two functions depending on whether this is a Left or Right value.
  pub fn either<U, F, G>(self, f: F, g: G) -> U
  where
    F: FnOnce(L) -> U,
    G: FnOnce(R) -> U,
  {
    match self {
      Either::Left(l) => f(l),
      Either::Right(r) => g(r),
    }
  }
}
