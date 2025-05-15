//! Implementation of the `Category` trait for `Either<L, R>`.
//!
//! This module provides a category implementation for a wrapper around the `Either` type.
//! The implementation supports the needs of the functor, applicative, and monad implementations.

use crate::traits::category::Category;
use crate::types::either::Either;
use crate::types::threadsafe::CloneableThreadSafe;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;

// Direct Category implementation for Either
impl<L: CloneableThreadSafe, R: CloneableThreadSafe> Category<R, R> for Either<L, R> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = Either<L, Arc<dyn Fn(&A) -> B + Send + Sync>>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    Either::Right(Arc::new(move |x: &A| x.clone()))
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>
  ) -> Self::Morphism<A, C> {
    match (f, g) {
      (Either::Left(l), _) => Either::Left(l),
      (_, Either::Left(l)) => Either::Left(l),
      (Either::Right(f_fn), Either::Right(g_fn)) => {
        let composed = Arc::new(move |a: &A| {
          let b = f_fn(a);
          g_fn(&b)
        });
        Either::Right(composed)
      }
    }
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(&A) -> B + CloneableThreadSafe,
  {
    Either::Right(Arc::new(f))
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>
  ) -> Self::Morphism<(A, C), (B, C)> {
    match f {
      Either::Left(l) => Either::Left(l),
      Either::Right(f_fn) => {
        let first_fn = Arc::new(move |pair: &(A, C)| {
          let (a, c) = pair;
          let b = f_fn(a);
          (b, c.clone())
        });
        Either::Right(first_fn)
      }
    }
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>
  ) -> Self::Morphism<(C, A), (C, B)> {
    match f {
      Either::Left(l) => Either::Left(l),
      Either::Right(f_fn) => {
        let second_fn = Arc::new(move |pair: &(C, A)| {
          let (c, a) = pair;
          let b = f_fn(a);
          (c.clone(), b)
        });
        Either::Right(second_fn)
      }
    }
  }
}

/// A wrapper type for implementing Category
#[derive(Clone)]
pub struct EitherCategory<L: CloneableThreadSafe>(PhantomData<L>);

impl<L: CloneableThreadSafe> EitherCategory<L> {
  pub fn new() -> Self {
    Self(PhantomData)
  }
}

// Create a newtype wrapper to handle functions
// Use Arc<dyn Fn> instead of Box<dyn Fn> to make cloning possible
#[derive(Clone)]
pub struct EitherFunction<L, A, B> {
  inner: Either<L, Arc<dyn Fn(&A) -> B + Send + Sync>>,
  _phantom: PhantomData<(L, A, B)>,
}

impl<L: CloneableThreadSafe + Debug, A: CloneableThreadSafe, B: CloneableThreadSafe> EitherFunction<L, A, B> {
  pub fn right<F>(f: F) -> Self 
  where
    F: Fn(&A) -> B + Clone + Send + Sync + 'static,
  {
    Self {
      inner: Either::Right(Arc::new(f)),
      _phantom: PhantomData,
    }
  }
  
  pub fn left(l: L) -> Self {
    Self {
      inner: Either::Left(l),
      _phantom: PhantomData,
    }
  }
  
  pub fn apply(&self, a: &A) -> Result<B, &L> {
    match &self.inner {
      Either::Left(l) => Err(l),
      Either::Right(f) => Ok(f(a)),
    }
  }
  
  pub fn is_left(&self) -> bool {
    matches!(self.inner, Either::Left(_))
  }
  
  pub fn is_right(&self) -> bool {
    matches!(self.inner, Either::Right(_))
  }
  
  pub fn get_left(&self) -> Option<&L> {
    match &self.inner {
      Either::Left(l) => Some(l),
      _ => None,
    }
  }
}

impl<L: CloneableThreadSafe + Debug + 'static> Category<L, L> for EitherCategory<L> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = EitherFunction<L, A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    EitherFunction::right(|x: &A| x.clone())
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>
  ) -> Self::Morphism<A, C> {
    if f.is_left() {
      return EitherFunction::left(f.get_left().unwrap().clone());
    }
    
    if g.is_left() {
      return EitherFunction::left(g.get_left().unwrap().clone());
    }
    
    EitherFunction::right(move |a: &A| {
      let b = f.apply(a).expect("Function application failed");
      g.apply(&b).expect("Function application failed")
    })
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(&A) -> B + CloneableThreadSafe,
  {
    EitherFunction::right(f)
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>
  ) -> Self::Morphism<(A, C), (B, C)> {
    if f.is_left() {
      return EitherFunction::left(f.get_left().unwrap().clone());
    }
    
    EitherFunction::right(move |pair: &(A, C)| {
      let (a, c) = pair;
      let b = f.apply(a).expect("Function application failed");
      (b, c.clone())
    })
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>
  ) -> Self::Morphism<(C, A), (C, B)> {
    if f.is_left() {
      return EitherFunction::left(f.get_left().unwrap().clone());
    }
    
    EitherFunction::right(move |pair: &(C, A)| {
      let (c, a) = pair;
      let b = f.apply(a).expect("Function application failed");
      (c.clone(), b)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Tests for Direct Category implementation for Either
  #[test]
  fn test_either_category_id() {
    let id = Either::<String, i32>::id::<i32>();
    match id {
      Either::Right(f) => {
        let value = 42;
        assert_eq!(f(&value), 42);
      }
      Either::Left(_) => panic!("Expected Right, got Left"),
    }
  }

  #[test]
  fn test_either_category_compose() {
    let add_one = Either::<String, i32>::arr(|x: &i32| x + 1);
    let double = Either::<String, i32>::arr(|x: &i32| x * 2);
    
    let composed = Either::<String, i32>::compose(add_one, double);
    
    match composed {
      Either::Right(f) => {
        let x = 5;
        assert_eq!(f(&x), 12); // (5+1)*2
      }
      Either::Left(_) => panic!("Expected Right, got Left"),
    }
  }

  #[test]
  fn test_either_category_left_propagation() {
    let left: Either<String, Arc<dyn Fn(&i32) -> i32 + Send + Sync>> = Either::Left("error".to_string());
    let right = Either::<String, i32>::arr(|x: &i32| x + 1);
    
    let result1 = Either::<String, i32>::compose(left.clone(), right.clone());
    let result2 = Either::<String, i32>::compose(right, left);
    
    assert!(matches!(result1, Either::Left(_)));
    assert!(matches!(result2, Either::Left(_)));
  }

  // Original tests for EitherCategory
  #[test]
  fn test_identity_law() {
    type EC = EitherCategory<String>;
    
    // Get the identity function
    let id = EC::id::<i32>();
    
    // Test identity on a value
    let value = 42;
    let result = id.apply(&value).unwrap();
    assert_eq!(result, 42);
  }

  #[test]
  fn test_composition_law() {
    type EC = EitherCategory<String>;
    
    // Define two functions
    let add_one = |x: &i32| x + 1;
    let double = |x: &i32| x * 2;
    
    // Create morphisms
    let add_one_morph = EC::arr(add_one);
    let double_morph = EC::arr(double);
    
    // Compose them
    let composed = EC::compose(add_one_morph, double_morph);
    
    // Apply and check result
    let x = 5;
    let result = composed.apply(&x).unwrap();
    assert_eq!(result, 12); // (5+1)*2
  }

  #[test]
  fn test_first_combinator() {
    type EC = EitherCategory<String>;
    
    // Create a morphism
    let inc = |x: &i32| x + 1;
    let inc_morph = EC::arr(inc);
    
    // Apply first combinator
    let first_inc = EC::first(inc_morph);
    
    // Apply and check result
    let pair = (5, "test".to_string());
    let result = first_inc.apply(&pair).unwrap();
    assert_eq!(result, (6, "test".to_string()));
  }

  #[test]
  fn test_second_combinator() {
    type EC = EitherCategory<String>;
    
    // Create a morphism
    let inc = |x: &i32| x + 1;
    let inc_morph = EC::arr(inc);
    
    // Apply second combinator
    let second_inc = EC::second(inc_morph);
    
    // Apply and check result
    let pair = ("test".to_string(), 5);
    let result = second_inc.apply(&pair).unwrap();
    assert_eq!(result, ("test".to_string(), 6));
  }

  #[test]
  fn test_left_propagation() {
    type EC = EitherCategory<String>;
    
    // Create a Left value
    let left = EitherFunction::<String, i32, i32>::left("error".to_string());
    
    // Try to compose with a Right value
    let inc = |x: &i32| x + 1;
    let right = EC::arr(inc);
    
    // Compose in both orders
    let result1 = EC::compose(left.clone(), right.clone());
    let result2 = EC::compose(right, left.clone());
    
    // Both compositions should result in Left
    assert!(result1.is_left());
    assert!(result2.is_left());
    
    assert_eq!(result1.get_left().unwrap(), "error");
    assert_eq!(result2.get_left().unwrap(), "error");
  }

  proptest! {
    #[test]
    fn prop_identity_law(x: i32) {
      type EC = EitherCategory<String>;
      
      let id = EC::id::<i32>();
      let result = id.apply(&x).unwrap();
      
      prop_assert_eq!(result, x);
    }
    
    #[test]
    fn prop_composition_law(x: i32) {
      type EC = EitherCategory<String>;
      
      let add = |a: &i32| a.saturating_add(1);
      let mul = |b: &i32| b.saturating_mul(2);
      
      let add_morph = EC::arr(add);
      let mul_morph = EC::arr(mul);
      
      let composed = EC::compose(add_morph, mul_morph);
      
      let direct = mul(&add(&x));
      let via_composed = composed.apply(&x).unwrap();
      
      prop_assert_eq!(direct, via_composed);
    }
  }
} 