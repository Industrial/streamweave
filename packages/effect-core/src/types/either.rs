//! Provides the [`Either`] enum for representing a value of one of two possible types.
//!
//! [`Either`] is a general-purpose sum type, similar to `Result<T, E>` but without the semantic meaning of success or error.
//! It is useful for representing computations or data that may take one of two forms.

/// A type representing a value of one of two possible types (a disjoint union).
///
/// This is similar to `Result<T, E>` but without the semantic meaning of success/error.
///
/// # Type Parameters
/// - `L`: The type of the left variant.
/// - `R`: The type of the right variant.
///
/// # Examples
/// ```
/// use effect_core::types::either::Either;
/// let left: Either<i32, &str> = Either::left(42);
/// let right: Either<i32, &str> = Either::right("hello");
/// assert!(left.is_left());
/// assert!(right.is_right());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Either<L, R> {
  /// The left variant of the Either type
  Left(L),
  /// The right variant of the Either type
  Right(R),
}

impl<L, R> Either<L, R> {
  /// Creates a new `Either::Left` value.
  ///
  /// # Arguments
  /// * `value` - The value to store in the left variant.
  pub fn left(value: L) -> Self {
    Either::Left(value)
  }

  /// Creates a new `Either::Right` value.
  ///
  /// # Arguments
  /// * `value` - The value to store in the right variant.
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
  ///
  /// # Arguments
  /// * `f` - The function to apply to the left value.
  ///
  /// # Returns
  /// A new `Either` with the mapped left value, or the original right value.
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
  ///
  /// # Arguments
  /// * `f` - The function to apply to the right value.
  ///
  /// # Returns
  /// A new `Either` with the mapped right value, or the original left value.
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
  ///
  /// # Arguments
  /// * `f` - Function to apply if this is a Left value.
  /// * `g` - Function to apply if this is a Right value.
  ///
  /// # Returns
  /// The result of applying the appropriate function to the contained value.
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
