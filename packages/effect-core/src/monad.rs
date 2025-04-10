//! Monad trait and implementations for the Effect type.
//!
//! This module defines the `Monad` trait and provides implementations for the
//! `Effect` type, enabling monadic composition and transformation of effects.

use std::collections::HashMap;
use std::hash::Hash;

/// The function type for bind operations.
pub type BindFn<T, U> = dyn FnOnce(T) -> U;

/// The Monad trait represents a type that can sequence computations.
pub trait Monad<T> {
  /// The higher-kinded type that results from sequencing computations
  type HigherSelf<U: Send + Sync + 'static>;

  /// Lifts a value into the monadic context
  fn pure(value: T) -> Self::HigherSelf<T>
  where
    T: Send + Sync + 'static;

  /// Sequences computations within the monadic context
  fn bind<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static;
}

// Implementation for Option
impl<A: Send + Sync + 'static> Monad<A> for Option<A> {
  type HigherSelf<U: Send + Sync + 'static> = Option<U>;

  fn pure(a: A) -> Self::HigherSelf<A> {
    Some(a)
  }

  fn bind<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    self.and_then(f)
  }
}

// Implementation for Vec
impl<A: Send + Sync + 'static> Monad<A> for Vec<A> {
  type HigherSelf<U: Send + Sync + 'static> = Vec<U>;

  fn pure(a: A) -> Self::HigherSelf<A> {
    vec![a]
  }

  fn bind<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    self.into_iter().flat_map(f).collect()
  }
}

// Implementation for Result
impl<A: Send + Sync + 'static, E> Monad<A> for Result<A, E> {
  type HigherSelf<U: Send + Sync + 'static> = Result<U, E>;

  fn pure(a: A) -> Self::HigherSelf<A> {
    Ok(a)
  }

  fn bind<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    self.and_then(f)
  }
}

// Implementation for Box
impl<A: Send + Sync + 'static> Monad<A> for Box<A> {
  type HigherSelf<U: Send + Sync + 'static> = Box<U>;

  fn pure(a: A) -> Self::HigherSelf<A> {
    Box::new(a)
  }

  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    f(*self)
  }
}

// Implementation for HashMap (values only)
impl<K, V> Monad<V> for HashMap<K, V>
where
  K: Clone + Eq + Hash + Default + Send + Sync + 'static,
  V: Send + Sync + 'static,
{
  type HigherSelf<U: Send + Sync + 'static> = HashMap<K, U>;

  fn pure(a: V) -> Self::HigherSelf<V> {
    let mut map = HashMap::new();
    map.insert(K::default(), a);
    map
  }

  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(V) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    self
      .into_iter()
      .flat_map(|(k, v)| f(v).into_values().map(move |b| (k.clone(), b)))
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::applicative::Applicative;
  use crate::functor::Functor;
  use std::fmt::Debug;

  // Define a simple monad for testing
  #[derive(Debug, PartialEq, Clone)]
  struct TestMonad<T>(T);

  impl<T> TestMonad<T> {
    fn new(value: T) -> Self {
      TestMonad(value)
    }
  }

  impl<T: Send + Sync + 'static> Functor<T> for TestMonad<T> {
    type HigherSelf<U: Send + Sync + 'static> = TestMonad<U>;

    fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
    where
      F: FnMut(T) -> U,
      U: Send + Sync + 'static,
    {
      TestMonad(f(self.0))
    }
  }

  impl<T: Send + Sync + 'static> Applicative<T> for TestMonad<T> {
    type HigherSelf<U: Send + Sync + 'static> = TestMonad<U>;

    fn pure(a: T) -> Self::HigherSelf<T> {
      TestMonad(a)
    }

    fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
    where
      F: FnMut(T) -> B + Send + Sync + 'static,
      B: Send + Sync + 'static,
    {
      let TestMonad(mut func) = f;
      let TestMonad(value) = self;
      TestMonad(func(value))
    }
  }

  impl<T: Send + Sync + 'static> Monad<T> for TestMonad<T> {
    type HigherSelf<U: Send + Sync + 'static> = TestMonad<U>;

    fn pure(a: T) -> Self::HigherSelf<T> {
      TestMonad(a)
    }

    fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
    where
      F: FnMut(T) -> Self::HigherSelf<B> + Send + Sync + 'static,
      B: Send + Sync + 'static,
    {
      f(self.0)
    }
  }

  // Test monad laws
  #[test]
  fn test_monad_laws() {
    // Left identity: pure(a).flat_map(f) == f(a)
    let a = 42;
    let f = |x: i32| TestMonad::new(x * 2);
    assert_eq!(<TestMonad<i32> as Monad<i32>>::pure(a).bind(f), f(a));

    // Right identity: m.flat_map(pure) == m
    let m = TestMonad::new(42);
    assert_eq!(m.clone().bind(<TestMonad<i32> as Monad<i32>>::pure), m);

    // Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
    let m = TestMonad::new(42);
    let f = |x: i32| TestMonad::new(x * 2);
    let g = |x: i32| TestMonad::new(x + 1);
    assert_eq!(m.clone().bind(f).bind(g), m.bind(move |x| f(x).bind(g)));
  }

  // Test basic operations
  #[test]
  fn test_unit() {
    let m: TestMonad<i32> = <TestMonad<i32> as Monad<i32>>::pure(42);
    assert_eq!(m, TestMonad(42));
  }

  #[test]
  fn test_bind() {
    let m: TestMonad<i32> =
      <TestMonad<i32> as Monad<i32>>::pure(42).bind(|x| TestMonad::new(x * 2));
    assert_eq!(m, TestMonad(84));
  }

  // Test type conversions
  #[test]
  fn test_type_conversions() {
    let m: TestMonad<String> =
      <TestMonad<i32> as Monad<i32>>::pure(42).bind(|x: i32| TestMonad::new(x.to_string()));
    assert_eq!(m, TestMonad("42".to_string()));
  }

  // Test complex composition
  #[test]
  fn test_complex_composition() {
    let m: TestMonad<String> = <TestMonad<i32> as Monad<i32>>::pure(1)
      .bind(|x: i32| <TestMonad<i32> as Monad<i32>>::pure(x + 1))
      .bind(|x: i32| <TestMonad<i32> as Monad<i32>>::pure(x * 2))
      .bind(|x: i32| <TestMonad<i32> as Monad<i32>>::pure(x + 1))
      .bind(|x: i32| TestMonad::new(x.to_string()));

    assert_eq!(m, TestMonad("5".to_string()));
  }

  // Test nested composition
  #[test]
  fn test_nested_composition() {
    let m: TestMonad<i32> = <TestMonad<i32> as Monad<i32>>::pure(1).bind(move |x: i32| {
      <TestMonad<i32> as Monad<i32>>::pure(x + 1).bind(move |y: i32| {
        <TestMonad<i32> as Monad<i32>>::pure(y * 2).bind(move |z: i32| TestMonad::new(z + x))
      })
    });

    assert_eq!(m, TestMonad(5));
  }

  mod option_tests {
    use super::*;

    #[test]
    fn test_unit() {
      let unit = <Option<i32> as Monad<i32>>::pure(42);
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
      let unit = <Vec<i32> as Monad<i32>>::pure(42);
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
