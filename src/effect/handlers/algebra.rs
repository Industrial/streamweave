//! Algebraic effects implementation.
//!
//! This module provides types and traits for implementing algebraic effects
//! in the effect system.

use std::any::Any;
use std::error::Error as StdError;
use std::future::Future;

use crate::effect::handlers::handler::{EffectHandler, HandlerContext};
use effect_core::effect::Effect;

/// A trait for effect operations.
pub trait Operation: Any + Send + Sync {
  /// The type of the operation's result.
  type Output: Send + Sync + 'static;
}

/// A trait for effect algebras.
pub trait Algebra {
  /// The type of error that can occur.
  type Error: StdError + Send + Sync + 'static;

  /// Performs an operation.
  fn perform<O: Operation>(&self, op: O) -> Effect<O::Output, Self::Error>;
}

/// A handler for algebraic effects.
pub struct AlgebraicHandler<A: Algebra> {
  algebra: A,
}

impl<A: Algebra> AlgebraicHandler<A> {
  /// Creates a new algebraic handler.
  pub fn new(algebra: A) -> Self {
    Self { algebra }
  }
}

impl<T, A> EffectHandler<T, A::Error> for AlgebraicHandler<A>
where
  T: Send + Sync + 'static,
  A: Algebra + Send + Sync + 'static,
{
  fn handle(&self, ctx: HandlerContext<T, A::Error>) -> Effect<T, A::Error> {
    ctx.effect()
  }
}

/// A basic operation that can be performed.
#[derive(Debug)]
pub struct BasicOperation<T>(pub T);

impl<T: Send + Sync + 'static> Operation for BasicOperation<T> {
  type Output = T;
}

/// A basic algebra that can perform operations.
pub struct BasicAlgebra;

impl Algebra for BasicAlgebra {
  type Error = std::io::Error;

  fn perform<O: Operation>(&self, op: O) -> Effect<O::Output, Self::Error> {
    Effect::pure(
      op.into_any()
        .downcast_ref::<BasicOperation<O::Output>>()
        .unwrap()
        .0
        .clone(),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};

  #[derive(Debug)]
  struct Add(i32);

  impl Operation for Add {
    type Output = i32;
  }

  struct TestAlgebra;

  impl Algebra for TestAlgebra {
    type Error = IoError;

    fn perform<O: Operation>(&self, op: O) -> Effect<O::Output, Self::Error> {
      if let Some(add) = op.into_any().downcast_ref::<Add>() {
        Effect::pure(add.0 + 1)
      } else {
        Effect::new(async { Err(IoError::new(ErrorKind::Other, "Unknown operation")) })
      }
    }
  }

  #[tokio::test]
  async fn test_algebraic_handler() {
    let algebra = TestAlgebra;
    let handler = AlgebraicHandler::new(algebra);
    let effect = Effect::pure(41);
    let ctx = HandlerContext::new(effect);
    let result = handler.handle(ctx).run().await.unwrap();
    assert_eq!(result, 41);
  }

  #[tokio::test]
  async fn test_perform_operation() {
    let algebra = TestAlgebra;
    let result = algebra.perform(Add(41)).run().await.unwrap();
    assert_eq!(result, 42);
  }

  #[tokio::test]
  async fn test_perform_unknown_operation() {
    struct Unknown;
    impl Operation for Unknown {
      type Output = ();
    }

    let algebra = TestAlgebra;
    let result = algebra.perform(Unknown).run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Unknown operation");
  }

  #[tokio::test]
  async fn test_basic_algebra() {
    let algebra = BasicAlgebra;
    let result = algebra.perform(BasicOperation(42)).run().await.unwrap();
    assert_eq!(result, 42);
  }
}
