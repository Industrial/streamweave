//! Either type implementation for error handling.
//!
//! This module provides the `Either` type, which represents a value that can be
//! either a left value or a right value. It is commonly used for error handling
//! in the effect system.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::bifunctor::Bifunctor;
use super::category::{Category, Morphism};
use crate::functor::{Functor, Mappable};
use crate::monad::Monad;
use std::marker::PhantomData;
use std::sync::Arc;

/// A type that represents either a left value or a right value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Either<L, R> {
  /// The left variant.
  Left(L),
  /// The right variant.
  Right(R),
}

impl<L, R> Either<L, R> {
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
  pub fn map_left<F, T>(self, f: F) -> Either<T, R>
  where
    F: FnOnce(L) -> T,
  {
    match self {
      Self::Left(l) => Either::Left(f(l)),
      Self::Right(r) => Either::Right(r),
    }
  }

  /// Maps the right value using the given function.
  pub fn map_right<F, T>(self, f: F) -> Either<L, T>
  where
    F: FnOnce(R) -> T,
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

impl<L, R> fmt::Display for Either<L, R>
where
  L: fmt::Display,
  R: fmt::Display,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Left(l) => write!(f, "Left({})", l),
      Self::Right(r) => write!(f, "Right({})", r),
    }
  }
}

impl<L: Send + Sync + 'static, R: Send + Sync + 'static> Functor<R> for Either<L, R> {
  type HigherSelf<U: Send + Sync + 'static> = Either<L, U>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(R) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    match self {
      Self::Left(l) => Either::Left(l),
      Self::Right(r) => Either::Right(f(r)),
    }
  }
}

impl<L: Send + Sync + 'static, R: Send + Sync + 'static> Monad<R> for Either<L, R> {
  type HigherSelf<U: Send + Sync + 'static> = Either<L, U>;

  fn pure(a: R) -> Self::HigherSelf<R> {
    Self::Right(a)
  }

  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(R) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    match self {
      Self::Left(l) => Either::Left(l),
      Self::Right(r) => f(r),
    }
  }
}

impl<L: Send + Sync + 'static, R: Send + Sync + 'static> Mappable<R> for Either<L, R> {}

impl<L, R> Future for Either<L, R>
where
  L: Future + Send + Sync + 'static,
  R: Future + Send + Sync + 'static,
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

impl<L, R> Category for Either<L, R> {
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

impl<L, R> Bifunctor for Either<L, R> {
  fn bimap<A, B, C, D, F, G>(f: F, g: G) -> Self::Morphism<(A, B), (C, D)>
  where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    C: Send + Sync + 'static,
    D: Send + Sync + 'static,
    F: Fn(A) -> C + Send + Sync + 'static,
    G: Fn(B) -> D + Send + Sync + 'static,
  {
    Morphism::new(move |(a, b)| (f(a), g(b)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::future::Future;
  use std::pin::Pin;
  use std::task::{Context, Poll};

  #[test]
  fn test_construction() {
    let left = Either::<i32, String>::left(42);
    assert!(left.is_left());
    assert!(!left.is_right());
    assert_eq!(left.unwrap_left(), 42);

    let right = Either::<i32, String>::right("hello".to_string());
    assert!(!right.is_left());
    assert!(right.is_right());
    assert_eq!(right.unwrap_right(), "hello");
  }

  #[test]
  fn test_map_left() {
    let left = Either::<i32, String>::left(42);
    let mapped = left.map_left(|x| x * 2);
    assert_eq!(mapped.unwrap_left(), 84);

    let right = Either::<i32, String>::right("hello".to_string());
    let mapped = right.map_left(|x| x * 2);
    assert_eq!(mapped.unwrap_right(), "hello");
  }

  #[test]
  fn test_map_right() {
    let left = Either::<i32, String>::left(42);
    let mapped = left.map_right(|s| s.len());
    assert_eq!(mapped.unwrap_left(), 42);

    let right = Either::<i32, String>::right("hello".to_string());
    let mapped = right.map_right(|s| s.len());
    assert_eq!(mapped.unwrap_right(), 5);
  }

  #[test]
  #[should_panic(expected = "called `Either::unwrap_left()` on a `Right` value")]
  fn test_unwrap_left_panic() {
    let right = Either::<i32, String>::right("hello".to_string());
    right.unwrap_left();
  }

  #[test]
  #[should_panic(expected = "called `Either::unwrap_right()` on a `Left` value")]
  fn test_unwrap_right_panic() {
    let left = Either::<i32, String>::left(42);
    left.unwrap_right();
  }

  #[test]
  fn test_display() {
    let left = Either::<i32, String>::left(42);
    assert_eq!(format!("{}", left), "Left(42)");

    let right = Either::<i32, String>::right("hello".to_string());
    assert_eq!(format!("{}", right), "Right(hello)");
  }

  #[test]
  fn test_clone() {
    let left = Either::<i32, String>::left(42);
    let cloned = left.clone();
    assert_eq!(left, cloned);

    let right = Either::<i32, String>::right("hello".to_string());
    let cloned = right.clone();
    assert_eq!(right, cloned);
  }

  #[test]
  fn test_debug() {
    let left = Either::<i32, String>::left(42);
    assert_eq!(format!("{:?}", left), "Left(42)");

    let right = Either::<i32, String>::right("hello".to_string());
    assert_eq!(format!("{:?}", right), "Right(\"hello\")");
  }

  #[test]
  fn test_partial_eq() {
    let left1 = Either::<i32, String>::left(42);
    let left2 = Either::<i32, String>::left(42);
    let left3 = Either::<i32, String>::left(43);
    assert_eq!(left1, left2);
    assert_ne!(left1, left3);

    let right1 = Either::<i32, String>::right("hello".to_string());
    let right2 = Either::<i32, String>::right("hello".to_string());
    let right3 = Either::<i32, String>::right("world".to_string());
    assert_eq!(right1, right2);
    assert_ne!(right1, right3);
  }

  #[test]
  fn test_complex_types() {
    let left = Either::<Vec<i32>, String>::left(vec![1, 2, 3]);
    assert_eq!(left.unwrap_left(), vec![1, 2, 3]);

    let right = Either::<Vec<i32>, String>::right("hello".to_string());
    assert_eq!(right.unwrap_right(), "hello");
  }

  #[test]
  fn test_nested_either() {
    let left = Either::<Either<i32, String>, f64>::left(Either::left(42));
    assert_eq!(left.unwrap_left().unwrap_left(), 42);

    let right = Either::<Either<i32, String>, f64>::right(std::f64::consts::PI);
    assert_eq!(right.unwrap_right(), std::f64::consts::PI);
  }

  #[test]
  fn test_zero_sized_types() {
    let left = Either::<(), ()>::left(());
    assert!(left.is_left());

    let right = Either::<(), ()>::right(());
    assert!(right.is_right());
  }

  #[test]
  fn test_reference_types() {
    let left = Either::<&str, String>::left("hello");
    assert_eq!(left.unwrap_left(), "hello");

    let right = Either::<&str, String>::right("world".to_string());
    assert_eq!(right.unwrap_right(), "world");
  }

  #[tokio::test]
  async fn test_future_impl() {
    #[derive(Clone, Copy)]
    struct TestFuture<T>(T);

    impl<T: Copy> Future for TestFuture<T> {
      type Output = T;

      fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.0)
      }
    }

    let left = Either::<TestFuture<i32>, TestFuture<i32>>::left(TestFuture(42));
    assert_eq!(left.await, Either::Left(42));

    let right = Either::<TestFuture<i32>, TestFuture<i32>>::right(TestFuture(99));
    assert_eq!(right.await, Either::Right(99));
  }

  proptest! {
    #[test]
    fn test_either_bifunctor_bimap(a: i32, b: i32) {
      let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
      let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
      let bimap = <Either<(), ()> as Bifunctor>::bimap(f, g);
      let expected = (
        a.checked_mul(2).unwrap_or(i32::MAX),
        b.checked_add(1).unwrap_or(i32::MAX)
      );
      assert_eq!(bimap.apply((a, b)), expected);
    }

    #[test]
    fn test_either_bifunctor_first(a: i32, b: i32) {
      let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
      let first = <Either<(), ()> as Bifunctor>::first(f);
      let expected = (a.checked_mul(2).unwrap_or(i32::MAX), b);
      assert_eq!(first.apply((a, b)), expected);
    }

    #[test]
    fn test_either_bifunctor_second(a: i32, b: i32) {
      let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
      let second = <Either<(), ()> as Bifunctor>::second(g);
      let expected = (a, b.checked_add(1).unwrap_or(i32::MAX));
      assert_eq!(second.apply((a, b)), expected);
    }

    #[test]
    fn test_either_bifunctor_laws(a: i32, b: i32) {
      let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
      let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
      let h = |x: i32| x.checked_mul(3).unwrap_or(i32::MAX);
      let k = |x: i32| x.checked_sub(1).unwrap_or(i32::MIN);

      // Test first . second = second . first
      let first_then_second = |(a, b)| {
        let first = <Either<(), ()> as Bifunctor>::first(f);
        let second = <Either<(), ()> as Bifunctor>::second(g);
        second.apply(first.apply((a, b)))
      };

      let second_then_first = |(a, b)| {
        let second = <Either<(), ()> as Bifunctor>::second(g);
        let first = <Either<(), ()> as Bifunctor>::first(f);
        first.apply(second.apply((a, b)))
      };

      let input = (a, b);
      let result1 = first_then_second(input.clone());
      let result2 = second_then_first(input);
      assert_eq!(result1, result2);

      // Test bimap composition
      let bimap1 = <Either<(), ()> as Bifunctor>::bimap(f, g);
      let bimap2 = <Either<(), ()> as Bifunctor>::bimap(h, k);
      let composed = |(a, b)| bimap2.apply(bimap1.apply((a, b)));

      let expected = (
        a.checked_mul(2)
          .and_then(|x| x.checked_mul(3))
          .unwrap_or(i32::MAX),
        b.checked_add(1)
          .and_then(|x| x.checked_sub(1))
          .unwrap_or(i32::MIN)
      );
      assert_eq!(composed((a, b)), expected);
    }
  }
}
