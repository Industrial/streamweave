//! Provides the [`Applicative`] trait for types that support lifting values and applying functions in a context.
//!
//! The [`Applicative`] trait extends [`Functor`] and allows for function application within a context, composition of effectful functions, and more structure than a plain functor.
//! It is a foundational abstraction in functional programming for working with computations in a context.

use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that are functors and support lifting values and applying functions in a context.
///
/// The [`Applicative`] trait provides the ability to lift pure values into a context (`pure`) and apply functions within that context (`ap`).
///
/// # Laws
///
/// 1. Identity: `pure(id).ap(v) == v`
/// 2. Composition: `pure(compose).ap(u).ap(v).ap(w) == u.ap(v.ap(w))`
/// 3. Homomorphism: `pure(f).ap(pure(x)) == pure(f(x))`
/// 4. Interchange: `u.ap(pure(y)) == pure(|f| f(y)).ap(u)`
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type parameter `A` and all related types must implement [`CloneableThreadSafe`].
pub trait Applicative<A: CloneableThreadSafe>: Functor<A> {
  /// Lifts a value into the applicative context.
  fn pure<B>(value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe;

  /// Applies a function within the applicative context.
  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe;
}
