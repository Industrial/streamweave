//! Either type implementation for error handling.
//!
//! This module provides the `Either` type, which represents a value that can be
//! either a left value or a right value. It is commonly used for error handling
//! in the effect system.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::applicative::Applicative;
use super::bifunctor::Bifunctor;
use super::category::{Category, Morphism};
use crate::functor::Functor;
use crate::monad::Monad;

/// A type that represents either a left value or a right value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Either<L: Send + Sync + 'static, R: Send + Sync + 'static> {
  /// The left variant.
  Left(L),
  /// The right variant.
  Right(R),
}

impl<L: Send + Sync + 'static, R: Send + Sync + 'static> Either<L, R> {
  /// Creates a new `Either` with a left value.
  pub fn left(value: L) -> Self {
    Self::Left(value)
  }

  /// Creates a new `Either` with a right value.
  pub fn right(value: R) -> Self {
    Self::Right(value)
  }

  /// Returns `true` if this is a left value.
  pub fn is_left(&self) -> bool {
    matches!(self, Self::Left(_))
  }

  /// Returns `true` if this is a right value.
  pub fn is_right(&self) -> bool {
    matches!(self, Self::Right(_))
  }

  /// Maps the left value using the given function.
  pub fn map_left<F, T: Send + Sync + 'static>(self, f: F) -> Either<T, R>
  where
    F: FnOnce(L) -> T + Send + Sync + 'static,
  {
    match self {
      Self::Left(l) => Either::Left(f(l)),
      Self::Right(r) => Either::Right(r),
    }
  }

  /// Maps the right value using the given function.
  pub fn map_right<F, T: Send + Sync + 'static>(self, f: F) -> Either<L, T>
  where
    F: FnOnce(R) -> T + Send + Sync + 'static,
  {
    match self {
      Self::Left(l) => Either::Left(l),
      Self::Right(r) => Either::Right(f(r)),
    }
  }

  /// Unwraps the left value, panicking if this is a right value.
  pub fn unwrap_left(self) -> L {
    match self {
      Self::Left(l) => l,
      Self::Right(_) => panic!("called `Either::unwrap_left()` on a `Right` value"),
    }
  }

  /// Unwraps the right value, panicking if this is a left value.
  pub fn unwrap_right(self) -> R {
    match self {
      Self::Left(_) => panic!("called `Either::unwrap_right()` on a `Left` value"),
      Self::Right(r) => r,
    }
  }
}

impl<L: Send + Sync + 'static + fmt::Display, R: Send + Sync + 'static + fmt::Display> fmt::Display
  for Either<L, R>
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Left(l) => write!(f, "Left({})", l),
      Self::Right(r) => write!(f, "Right({})", r),
    }
  }
}

impl<L: Send + Sync + 'static, R: Send + Sync + 'static> Category<R, R> for Either<L, R> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Morphism<A, B>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    Morphism::new(|x| x)
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    Morphism::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Morphism::new(f)
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    Morphism::new(move |(a, c)| (f.apply(a), c))
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    Morphism::new(move |(c, a)| (c, f.apply(a)))
  }
}

impl<L: Send + Sync + 'static, R: Send + Sync + 'static> Functor<R> for Either<L, R> {
  type HigherSelf<U: Send + Sync + 'static> = Either<L, U>;

  fn map<B: Send + Sync + 'static, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(R) -> B + Send + Sync + 'static,
  {
    match self {
      Self::Left(l) => Either::Left(l),
      Self::Right(r) => Either::Right(f(r)),
    }
  }
}

impl<L: Send + Sync + 'static, R: Send + Sync + 'static> Applicative<R> for Either<L, R> {
  type ApplicativeSelf<U: Send + Sync + 'static> = Either<L, U>;

  fn pure<A: Send + Sync + 'static>(a: A) -> Self::ApplicativeSelf<A> {
    Either::Right(a)
  }

  fn ap<U, F>(self, f: Self::ApplicativeSelf<F>) -> Self::ApplicativeSelf<U>
  where
    U: Send + Sync + 'static,
    F: FnOnce(R) -> U + Send + Sync + 'static,
  {
    match (self, f) {
      (Either::Right(a), Either::Right(func)) => Either::Right(func(a)),
      (Either::Left(l), _) => Either::Left(l),
      (_, Either::Left(l)) => Either::Left(l),
    }
  }
}

impl<L: Send + Sync + 'static, R: Send + Sync + 'static> Monad<R> for Either<L, R> {
  type MonadSelf<U: Send + Sync + 'static> = Either<L, U>;

  fn bind<B: Send + Sync + 'static, F>(self, mut f: F) -> Self::MonadSelf<B>
  where
    F: FnMut(R) -> Self::MonadSelf<B> + Send + Sync + 'static,
  {
    match self {
      Either::Left(l) => Either::Left(l),
      Either::Right(r) => f(r),
    }
  }
}

impl<L: Send + Sync + 'static, R: Send + Sync + 'static> Future for Either<L, R>
where
  L: Future<Output: Send + Sync + 'static> + Send + Sync + 'static,
  R: Future<Output: Send + Sync + 'static> + Send + Sync + 'static,
{
  type Output = Either<L::Output, R::Output>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    unsafe {
      match self.get_unchecked_mut() {
        Either::Left(l) => {
          let pinned = Pin::new_unchecked(l);
          pinned.poll(cx).map(Either::Left)
        }
        Either::Right(r) => {
          let pinned = Pin::new_unchecked(r);
          pinned.poll(cx).map(Either::Right)
        }
      }
    }
  }
}

impl<L: Send + Sync + 'static, R: Send + Sync + 'static> Bifunctor<L, R> for Either<L, R> {
  fn bimap<C: Send + Sync + 'static, D: Send + Sync + 'static, F, G>(
    f: F,
    g: G,
  ) -> Self::Morphism<(L, R), (C, D)>
  where
    F: Fn(L) -> C + Send + Sync + 'static,
    G: Fn(R) -> D + Send + Sync + 'static,
  {
    Morphism::new(move |(x, y)| (f(x), g(y)))
  }
}
