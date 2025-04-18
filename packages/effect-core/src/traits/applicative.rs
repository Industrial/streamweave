use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

/// An applicative functor is a functor that can lift values and apply functions.
///
/// Applicative functors allow function application within a context, composition of
/// effectful functions, and are more structured than Functors but less powerful than Monads.
///
/// # Laws
///
/// 1. Identity: `pure(id).ap(v) == v`
/// 2. Composition: `pure(compose).ap(u).ap(v).ap(w) == u.ap(v.ap(w))`
/// 3. Homomorphism: `pure(f).ap(pure(x)) == pure(f(x))`
/// 4. Interchange: `u.ap(pure(y)) == pure(|f| f(y)).ap(u)`
///
/// # Type Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter A must implement CloneableThreadSafe
/// - The higher-kinded type HigherSelf<B> must implement CloneableThreadSafe
/// - The function type F must implement CloneableThreadSafe
/// - The result type B must implement CloneableThreadSafe
pub trait Applicative<A: CloneableThreadSafe>: Functor<A> {
  /// Lifts a value into the applicative context
  fn pure<B>(value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe;

  /// Applies a function within the applicative context
  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe;
}
