//! Monad trait and implementations for the Effect type.
//!
//! This module defines the `Monad` trait and provides implementations for the
//! `Effect` type, enabling monadic composition and transformation of effects.

use std::collections::HashMap;

use super::applicative::Applicative;
use super::functor::Functor;

/// The function type for bind operations.
pub type BindFn<T, U> = dyn FnOnce(T) -> U;

/// The Monad trait defines the basic operations for monadic types.
pub trait Monad<A>: Applicative<A> {
  /// The type constructor for the monad.
  type SelfTrait<T>: Monad<T>;

  /// The type constructor for the unit operation.
  type Unit<T>: Monad<T>;

  /// The type constructor for the bind operation.
  type Bind<T, F>: Monad<T>;

  /// Lifts a value into the monadic context.
  fn unit(a: A) -> Self::Unit<A>;

  /// Binds a function over the monadic value.
  fn bind<B, F>(self, f: F) -> Self::Bind<B, F>
  where
    F: FnOnce(A) -> Self::SelfTrait<B>;
}

// Implementation for Option
impl<A> Monad<A> for Option<A> {
  type SelfTrait<T> = Option<T>;
  type Unit<T> = Option<T>;
  type Bind<T, F> = Option<T>;

  fn unit(value: A) -> Self::Unit<A> {
    Some(value)
  }

  fn bind<B, F>(self, f: F) -> Self::Bind<B, F>
  where
    F: FnMut(A) -> Self::SelfTrait<B>,
  {
    self.and_then(f)
  }
}

// Implementation for Vec
impl<A> Monad<A> for Vec<A> {
  type SelfTrait<T> = Vec<T>;
  type Unit<T> = Vec<T>;
  type Bind<T, F> = Vec<T>;

  fn unit(value: A) -> Self::Unit<A> {
    vec![value]
  }

  fn bind<B, F>(self, f: F) -> Self::Bind<B, F>
  where
    F: FnMut(A) -> Self::SelfTrait<B>,
  {
    self.into_iter().flat_map(f).collect()
  }
}

// Implementation for Result
impl<A, E> Monad<A> for Result<A, E> {
  type SelfTrait<T> = Result<T, E>;
  type Unit<T> = Result<T, E>;
  type Bind<T, F> = Result<T, E>;

  fn unit(value: A) -> Self::Unit<A> {
    Ok(value)
  }

  fn bind<B, F>(self, f: F) -> Self::Bind<B, F>
  where
    F: FnMut(A) -> Self::SelfTrait<B>,
  {
    self.and_then(f)
  }
}

// Implementation for Box
impl<A> Monad<A> for Box<A> {
  type SelfTrait<T> = Box<T>;
  type Unit<T> = Box<T>;
  type Bind<T, F> = Box<T>;

  fn unit(value: A) -> Self::Unit<A> {
    Box::new(value)
  }

  fn bind<B, F>(self, mut f: F) -> Self::Bind<B, F>
  where
    F: FnMut(A) -> Self::SelfTrait<B>,
  {
    f(*self)
  }
}

// Implementation for HashMap (values only)
impl<K: std::hash::Hash + Eq + std::default::Default + std::clone::Clone, V> Monad<V>
  for HashMap<K, V>
{
  type SelfTrait<T> = HashMap<K, T>;
  type Unit<T> = HashMap<K, T>;
  type Bind<T, F> = HashMap<K, T>;

  fn unit(value: V) -> Self::Unit<V> {
    let mut map = HashMap::new();
    map.insert(K::default(), value);
    map
  }

  fn bind<B, F>(self, mut f: F) -> Self::Bind<B, F>
  where
    F: FnMut(V) -> Self::SelfTrait<B>,
  {
    self
      .into_iter()
      .flat_map(|(k, v)| f(v).into_iter().map(move |(_, b)| (k.clone(), b)))
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fmt::Debug;

  // Define a simple monad for testing
  #[derive(Debug, PartialEq, Clone)]
  struct TestMonad<T>(T);

  impl<T> TestMonad<T> {
    fn new(value: T) -> Self {
      TestMonad(value)
    }

    fn of<U>(value: U) -> TestMonad<U> {
      TestMonad(value)
    }
  }

  impl<T> Functor<T> for TestMonad<T> {
    type HigherSelf<U> = TestMonad<U>;

    fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
    where
      F: FnMut(T) -> U,
    {
      TestMonad(f(self.0))
    }
  }

  impl<T> Applicative<T> for TestMonad<T> {
    fn pure(a: T) -> Self {
      TestMonad(a)
    }

    fn ap<B, F>(self, f: TestMonad<F>) -> TestMonad<B>
    where
      F: FnOnce(T) -> B,
    {
      let TestMonad(func) = f;
      let TestMonad(value) = self;
      TestMonad(func(value))
    }
  }

  impl<T> Monad<T> for TestMonad<T> {
    type SelfTrait<U> = TestMonad<U>;
    type Unit<U> = TestMonad<U>;
    type Bind<U, F> = TestMonad<U>;

    fn unit(a: T) -> Self::Unit<T> {
      TestMonad(a)
    }

    fn bind<U, F>(self, f: F) -> Self::Bind<U, F>
    where
      F: FnOnce(T) -> Self::SelfTrait<U>,
    {
      let TestMonad(u) = f(self.0);
      TestMonad(u)
    }
  }

  // Test monad laws
  #[test]
  fn test_monad_laws() {
    // Left identity: pure(a).flat_map(f) == f(a)
    let a = 42;
    let f = |x: i32| TestMonad::new(x * 2);
    assert_eq!(TestMonad::unit(a).bind(f), f(a));

    // Right identity: m.flat_map(pure) == m
    let m = TestMonad::new(42);
    assert_eq!(m.clone().bind(TestMonad::unit), m);

    // Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
    let m = TestMonad::new(42);
    let f = |x: i32| TestMonad::new(x * 2);
    let g = |x: i32| TestMonad::new(x + 1);
    assert_eq!(m.clone().bind(f).bind(g), m.bind(move |x| f(x).bind(g)));
  }

  // Test basic operations
  #[test]
  fn test_unit() {
    let m: TestMonad<i32> = TestMonad::unit(42);
    assert_eq!(m, TestMonad(42));
  }

  #[test]
  fn test_bind() {
    let m: TestMonad<i32> = TestMonad::unit(42).bind(|x| TestMonad::new(x * 2));
    assert_eq!(m, TestMonad(84));
  }

  // Test type conversions
  #[test]
  fn test_type_conversions() {
    let m: TestMonad<String> = TestMonad::of(42).bind(|x: i32| TestMonad::new(x.to_string()));
    assert_eq!(m, TestMonad("42".to_string()));
  }

  // Test complex composition
  #[test]
  fn test_complex_composition() {
    let m: TestMonad<String> = TestMonad::of(1)
      .bind(|x: i32| TestMonad::of(x + 1))
      .bind(|x: i32| TestMonad::of(x * 2))
      .bind(|x: i32| TestMonad::of(x + 1))
      .bind(|x: i32| TestMonad::new(x.to_string()));

    assert_eq!(m, TestMonad("5".to_string()));
  }

  // Test nested composition
  #[test]
  fn test_nested_composition() {
    let m: TestMonad<i32> = TestMonad::of(1).bind(move |x: i32| {
      TestMonad::of(x + 1)
        .bind(move |y: i32| TestMonad::of(y * 2).bind(move |z: i32| TestMonad::new(z + x)))
    });

    assert_eq!(m, TestMonad(5));
  }

  mod option_tests {
    use super::*;

    #[test]
    fn test_unit() {
      let unit = Option::<i32>::unit(42);
      assert_eq!(unit, Some(42));
    }

    #[test]
    fn test_bind_some() {
      let some = Some(42);
      let result = some.bind(|x| Some(x * 2));
      assert_eq!(result, Some(84));
    }

    #[test]
    fn test_bind_none() {
      let none: Option<i32> = None;
      let result = none.bind(|x| Some(x * 2));
      assert_eq!(result, None);
    }

    #[test]
    fn test_bind_composition() {
      let some = Some(42);
      let result = some.bind(|x| Some(x * 2)).bind(|x| Some(x + 1));
      assert_eq!(result, Some(85));
    }
  }

  mod vec_tests {
    use super::*;

    #[test]
    fn test_unit() {
      let unit = Vec::<i32>::unit(42);
      assert_eq!(unit, vec![42]);
    }

    #[test]
    fn test_bind_empty() {
      let empty: Vec<i32> = vec![];
      let result = empty.bind(|x| vec![x * 2]);
      assert_eq!(result, vec![]);
    }

    #[test]
    fn test_bind_single() {
      let vec = vec![42];
      let result = vec.bind(|x| vec![x * 2]);
      assert_eq!(result, vec![84]);
    }

    #[test]
    fn test_bind_multiple() {
      let vec = vec![1, 2, 3];
      let result = vec.bind(|x| vec![x * 2, x * 3]);
      assert_eq!(result, vec![2, 3, 4, 6, 6, 9]);
    }

    #[test]
    fn test_bind_composition() {
      let vec = vec![1, 2, 3];
      let result = vec.bind(|x| vec![x * 2]).bind(|x| vec![x + 1]);
      assert_eq!(result, vec![3, 5, 7]);
    }
  }
}
