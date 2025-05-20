//! Implementation of the `Arrow` trait for `Either<L, R>`.
//!
//! This module provides arrow implementations for the `Either` type, enabling
//! arrow operations on values that can be one of two types.

use crate::impls::either::category::EitherCategory;
use crate::impls::either::category::EitherFunction;
use crate::traits::arrow::Arrow;
use crate::types::either::Either;
use crate::types::threadsafe::CloneableThreadSafe;
use std::fmt::Debug;
use std::sync::Arc;

// Direct Arrow implementation for Either
impl<L: CloneableThreadSafe, R: CloneableThreadSafe> Arrow<R, R> for Either<L, R> {
  fn arrow<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + CloneableThreadSafe,
  {
    Either::Right(Arc::new(move |a: &A| f(a.clone())))
  }

  fn split<
    A: CloneableThreadSafe,
    B: CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  >(
    f: Self::Morphism<R, R>,
    g: Self::Morphism<A, B>,
  ) -> Self::Morphism<(R, A), (R, B)> {
    match (f, g) {
      (Either::Left(l), _) => Either::Left(l),
      (_, Either::Left(l)) => Either::Left(l),
      (Either::Right(f_fn), Either::Right(g_fn)) => {
        let split_fn = Arc::new(move |pair: &(R, A)| {
          let (r, a) = pair;
          (f_fn(r), g_fn(a))
        });
        Either::Right(split_fn)
      }
    }
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<R, R>,
    g: Self::Morphism<R, C>,
  ) -> Self::Morphism<R, (R, C)> {
    match (f, g) {
      (Either::Left(l), _) => Either::Left(l),
      (_, Either::Left(l)) => Either::Left(l),
      (Either::Right(f_fn), Either::Right(g_fn)) => {
        let fanout_fn = Arc::new(move |r: &R| {
          (f_fn(r), g_fn(r))
        });
        Either::Right(fanout_fn)
      }
    }
  }
}

// Arrow implementation for EitherCategory
impl<L: CloneableThreadSafe + Debug + 'static> Arrow<L, L> for EitherCategory<L> {
  fn arrow<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + CloneableThreadSafe,
  {
    EitherFunction::right(move |a: &A| f(a.clone()))
  }

  fn split<
    A: CloneableThreadSafe,
    B: CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  >(
    f: Self::Morphism<L, L>,
    g: Self::Morphism<A, B>,
  ) -> Self::Morphism<(L, A), (L, B)> {
    if f.is_left() {
      return EitherFunction::left(f.get_left().unwrap().clone());
    }

    if g.is_left() {
      return EitherFunction::left(g.get_left().unwrap().clone());
    }

    EitherFunction::right(move |pair: &(L, A)| {
      let (l, a) = pair;
      let new_l = f.apply(l).expect("Function application failed");
      let new_a = g.apply(a).expect("Function application failed");
      (new_l, new_a)
    })
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<L, L>,
    g: Self::Morphism<L, C>,
  ) -> Self::Morphism<L, (L, C)> {
    if f.is_left() {
      return EitherFunction::left(f.get_left().unwrap().clone());
    }

    if g.is_left() {
      return EitherFunction::left(g.get_left().unwrap().clone());
    }

    EitherFunction::right(move |l: &L| {
      let new_l = f.apply(l).expect("Function application failed");
      let c = g.apply(l).expect("Function application failed");
      (new_l, c)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Tests for Direct Arrow implementation for Either

  #[test]
  fn test_either_arrow() {
    let add_one = Either::<String, i32>::arrow(|x: i32| x + 1);
    match add_one {
      Either::Right(f) => {
        let value = 42;
        assert_eq!(f(&value), 43);
      }
      Either::Left(_) => panic!("Expected Right, got Left"),
    }
  }

  #[test]
  fn test_either_split() {
    let add_one = Either::<String, i32>::arrow(|x: i32| x + 1);
    let double = Either::<String, i32>::arrow(|x: i32| x * 2);

    let split = Either::<String, i32>::split::<i32, i32, (), ()>(add_one, double);

    match split {
      Either::Right(f) => {
        let pair = (5, 3);
        let result = f(&pair);
        assert_eq!(result, (6, 6)); // (5+1, 3*2)
      }
      Either::Left(_) => panic!("Expected Right, got Left"),
    }
  }

  #[test]
  fn test_either_fanout() {
    let add_one = Either::<String, i32>::arrow(|x: i32| x + 1);
    let double = Either::<String, i32>::arrow(|x: i32| x * 2);

    let fanout = Either::<String, i32>::fanout(add_one, double);

    match fanout {
      Either::Right(f) => {
        let value = 5;
        let result = f(&value);
        assert_eq!(result, (6, 10)); // (5+1, 5*2)
      }
      Either::Left(_) => panic!("Expected Right, got Left"),
    }
  }

  #[test]
  fn test_either_left_propagation_in_arrow() {
    let left: Either<String, Arc<dyn Fn(&i32) -> i32 + Send + Sync>> =
      Either::Left("error".to_string());
    let right = Either::<String, i32>::arrow(|x: i32| x + 1);

    let result1 = Either::<String, i32>::split::<i32, i32, (), ()>(left.clone(), right.clone());
    let result2 = Either::<String, i32>::split::<i32, i32, (), ()>(right.clone(), left.clone());
    let result3 = Either::<String, i32>::fanout(left, right);

    assert!(matches!(result1, Either::Left(_)));
    assert!(matches!(result2, Either::Left(_)));
    assert!(matches!(result3, Either::Left(_)));
  }

  // Tests for EitherCategory Arrow implementation

  #[test]
  fn test_either_category_arrow() {
    type EC = EitherCategory<String>;
    
    let add_one = EC::arrow(|x: String| x + "a");
    let result = add_one.apply(&"hello".to_string());
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "helloa");
  }
  
  #[test]
  fn test_either_category_split() {
    type EC = EitherCategory<String>;
    
    let add_a = EC::arrow(|x: String| x + "a");
    let add_b = EC::arrow(|x: String| x + "b");
    
    let split = EC::split::<String, String, (), ()>(add_a, add_b);
    let result = split.apply(&("hello".to_string(), "world".to_string()));
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ("helloa".to_string(), "worldb".to_string()));
  }
  
  #[test]
  fn test_either_category_fanout() {
    type EC = EitherCategory<String>;
    
    let add_a = EC::arrow(|x: String| x + "a");
    let add_b = EC::arrow(|x: String| x + "b");
    
    let fanout = EC::fanout(add_a, add_b);
    let result = fanout.apply(&"hello".to_string());
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ("helloa".to_string(), "hellob".to_string()));
  }
  
  #[test]
  fn test_either_category_left_propagation() {
    type EC = EitherCategory<String>;
    
    let left = EitherFunction::<String, String, String>::left("error".to_string());
    let right = EC::arrow(|x: String| x + "a");
    
    let split = EC::split::<String, String, (), ()>(left.clone(), right.clone());
    assert!(split.apply(&("hello".to_string(), "world".to_string())).is_err());
    
    let split2 = EC::split::<String, String, (), ()>(right.clone(), left.clone());
    assert!(split2.apply(&("hello".to_string(), "world".to_string())).is_err());
    
    let fanout = EC::fanout(left, right);
    assert!(fanout.apply(&"hello".to_string()).is_err());
  }

  // Test Arrow laws

  #[test]
  fn test_arrow_law_split_preserves_arrows() {
    // Arrow law: split preserves arrows
    // split(arr(f), arr(g)) = arr(\(x,y) -> (f x, g y))
    
    let f = |x: i32| x + 10;
    let g = |y: i32| y * 2;
    
    let arrow_f = Either::<String, i32>::arrow(f);
    let arrow_g = Either::<String, i32>::arrow(g);
    
    let split_arrows = Either::<String, i32>::split::<i32, i32, (), ()>(arrow_f, arrow_g);
    
    let combined_fn = move |pair: (i32, i32)| (f(pair.0), g(pair.1));
    let arrow_combined = Either::<String, (i32, i32)>::arrow(combined_fn);
    
    // Test by applying both arrows to the same input
    let input = (5, 7);
    
    match (split_arrows, arrow_combined) {
      (Either::Right(split_fn), Either::Right(combined_fn)) => {
        assert_eq!(split_fn(&input), combined_fn(&input));
      }
      _ => panic!("Expected Right variants for both arrows"),
    }
  }
} 