//! Provides the [`Monad`] trait for types that support sequential composition of computations.
//!
//! The [`Monad`] trait extends [`Applicative`] and represents a computation with a context that
//! can be sequentially composed. It provides a way to chain operations that depend on previous results
//! while preserving the computational context.
//!
//! This trait is essentially the Rust equivalent of the `Monad` typeclass in Haskell, enabling powerful
//! functional programming patterns for handling effects like optionality, errors, and asynchronous operations.

use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that support sequential composition of computations with context.
///
/// A [`Monad`] represents a computation that has a context or effect, such as optionality (`Option`),
/// error handling (`Result`), or asynchronous operations (`Future`).
///
/// The trait provides the fundamental `bind` operation (also known as "flatMap" or ">>=" in Haskell),
/// which allows chaining operations in a way that:
/// 1. Takes a value in a monadic context
/// 2. Applies a function that returns a new value in a monadic context
/// 3. Flattens the nested monadic contexts into a single context
///
/// This enables clean, composable error handling, control flow, and effects management.
///
/// # Laws
///
/// A proper [`Monad`] implementation must satisfy the following laws:
///
/// 1. Left identity: `pure(a).bind(f) == f(a)`
///    - Lifting a value with `pure` and then binding with function `f` should be the same as applying `f` directly.
///
/// 2. Right identity: `m.bind(pure) == m`
///    - Binding a monadic value `m` with the `pure` function should return `m` unchanged.
///
/// 3. Associativity: `m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))`
///    - The order of nested binds shouldn't matter.
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type parameter `A` and all related types must implement [`CloneableThreadSafe`].
pub trait Monad<A: CloneableThreadSafe>: Applicative<A> {
  /// The core monadic bind operation, also known as "flatMap" or ">>=" in Haskell.
  ///
  /// Applies a function that returns a monadic value to this monadic value and flattens the result.
  /// 
  /// This is the fundamental operation for sequencing computations that return values in a context.
  /// It allows for the composition of operations where each operation depends on the result of the previous one.
  ///
  /// # Examples
  ///
  /// ```
  /// use effect_core::traits::monad::Monad;
  /// use effect_core::traits::applicative::Applicative;
  ///
  /// // Using Option as a monad to handle a chain of operations that might fail
  /// let result = Some(3)
  ///     .bind(|x| if *x > 0 { Some(100 / x) } else { None })
  ///     .bind(|x| if *x < 50 { Some(x * 2) } else { None });
  ///
  /// assert_eq!(result, Some(66)); // (100 / 3) * 2 = 66
  /// ```
  fn bind<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe;

  /// Sequences two monadic actions, discarding the value of the first but keeping its effects.
  ///
  /// This is useful for chaining operations where you don't need the result of the first operation,
  /// but you still want its effects (like error propagation or state changes) to be considered.
  ///
  /// This is equivalent to Haskell's `>>` operator.
  ///
  /// # Examples
  ///
  /// ```
  /// use effect_core::traits::monad::Monad;
  /// use effect_core::traits::applicative::Applicative;
  ///
  /// // Using Option to sequence operations, discarding the first result
  /// let result = Some(10).then(Some(20));
  /// assert_eq!(result, Some(20));
  ///
  /// // If the first operation is None, the second is ignored
  /// let result = None::<i32>.then(Some(20));
  /// assert_eq!(result, None);
  /// ```
  fn then<B>(self, mb: Self::HigherSelf<B>) -> Self::HigherSelf<B>
  where
    Self: Sized,
    B: CloneableThreadSafe,
  {
    self.bind(move |_| mb.clone())
  }

  /// An alias for `bind` to match common functional programming terminology.
  ///
  /// This method behaves identically to `bind`, but provides a name that might be
  /// more familiar to users of Scala, Kotlin, or other functional programming libraries.
  ///
  /// # Examples
  ///
  /// ```
  /// use effect_core::traits::monad::Monad;
  /// use effect_core::traits::applicative::Applicative;
  ///
  /// // Using flat_map with Option
  /// let result = Some(5).flat_map(|x| Some(x * 2));
  /// assert_eq!(result, Some(10));
  /// ```
  fn flat_map<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    Self: Sized,
    F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    self.bind(f)
  }
}
