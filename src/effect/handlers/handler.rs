//! Effect handler implementation.
//!
//! This module provides the `EffectHandler` trait and related types for
//! handling effects in the effect system.

use std::error::Error as StdError;
use std::future::Future;
use std::sync::{Arc, Mutex};

use crate::effect::utils::shared::Shared;
use effect_core::effect::Effect;

/// Context for handling effects.
pub struct HandlerContext<T, E: StdError + Send + Sync + 'static> {
  /// The current effect being handled.
  effect: Effect<T, E>,
}

impl<T, E: StdError + Send + Sync + 'static> HandlerContext<T, E> {
  /// Creates a new handler context.
  pub fn new(effect: Effect<T, E>) -> Self {
    Self { effect }
  }

  /// Gets the current effect.
  pub fn effect(&self) -> &Effect<T, E> {
    &self.effect
  }
}

/// A trait for handling effects.
pub trait EffectHandler<T, E: StdError + Send + Sync + 'static> {
  /// Handles an effect.
  fn handle(&self, ctx: HandlerContext<T, E>) -> Effect<T, E>;

  /// Composes this handler with another handler.
  fn and_then<H>(self, other: H) -> ComposedHandler<Self, H>
  where
    Self: Sized,
    H: EffectHandler<T, E>,
  {
    ComposedHandler {
      first: self,
      second: other,
    }
  }
}

/// A handler that composes two handlers.
pub struct ComposedHandler<F, S> {
  first: F,
  second: S,
}

impl<T, E, F, S> EffectHandler<T, E> for ComposedHandler<F, S>
where
  E: StdError + Send + Sync + 'static,
  F: EffectHandler<T, E>,
  S: EffectHandler<T, E>,
{
  fn handle(&self, ctx: HandlerContext<T, E>) -> Effect<T, E> {
    let first_result = self.first.handle(ctx);
    let second_ctx = HandlerContext::new(first_result);
    self.second.handle(second_ctx)
  }
}

/// A handler that can be shared between threads.
pub type SharedHandler<T, E> = Arc<dyn EffectHandler<T, E> + Send + Sync>;

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};

  struct TestHandler;

  impl EffectHandler<i32, IoError> for TestHandler {
    fn handle(&self, ctx: HandlerContext<i32, IoError>) -> Effect<i32, IoError> {
      ctx.effect().map(|x| x * 2)
    }
  }

  struct AddHandler(i32);

  impl EffectHandler<i32, IoError> for AddHandler {
    fn handle(&self, ctx: HandlerContext<i32, IoError>) -> Effect<i32, IoError> {
      ctx.effect().map(|x| x + self.0)
    }
  }

  struct ErrorHandler;

  impl EffectHandler<i32, IoError> for ErrorHandler {
    fn handle(&self, ctx: HandlerContext<i32, IoError>) -> Effect<i32, IoError> {
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "handler error")) })
    }
  }

  #[tokio::test]
  async fn test_handler() {
    let handler = TestHandler;
    let effect = Effect::pure(21);
    let ctx = HandlerContext::new(effect);
    let result = handler.handle(ctx).run().await.unwrap();
    assert_eq!(result, 42);
  }

  #[tokio::test]
  async fn test_composed_handler() {
    let handler = TestHandler.and_then(AddHandler(1));
    let effect = Effect::pure(20);
    let ctx = HandlerContext::new(effect);
    let result = handler.handle(ctx).run().await.unwrap();
    assert_eq!(result, 41);
  }

  #[tokio::test]
  async fn test_handler_error() {
    let handler = TestHandler.and_then(ErrorHandler);
    let effect = Effect::pure(20);
    let ctx = HandlerContext::new(effect);
    let result = handler.handle(ctx).run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "handler error");
  }

  #[tokio::test]
  async fn test_shared_handler() {
    let handler: SharedHandler<i32, IoError> = Arc::new(TestHandler);
    let effect = Effect::pure(21);
    let ctx = HandlerContext::new(effect);
    let result = handler.handle(ctx).run().await.unwrap();
    assert_eq!(result, 42);
  }
}
