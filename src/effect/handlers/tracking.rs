//! Effect tracking implementation.
//!
//! This module provides types and traits for tracking effects as they are
//! executed, including logging and monitoring.

use std::error::Error as StdError;
use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

use crate::effect::core::effect::Effect;
use crate::effect::handlers::handler::{EffectHandler, HandlerContext};

/// Context for tracking effects.
pub struct TrackingContext {
  /// The start time of the effect.
  start_time: Instant,
  /// The name of the effect.
  name: String,
  /// Additional metadata about the effect.
  metadata: Arc<Mutex<Vec<(String, String)>>>,
}

impl TrackingContext {
  /// Creates a new tracking context.
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      start_time: Instant::now(),
      name: name.into(),
      metadata: Arc::new(Mutex::new(Vec::new())),
    }
  }

  /// Adds metadata to the context.
  pub fn add_metadata(&self, key: impl Into<String>, value: impl Into<String>) {
    self
      .metadata
      .lock()
      .unwrap()
      .push((key.into(), value.into()));
  }

  /// Gets the duration since the effect started.
  pub fn duration(&self) -> std::time::Duration {
    self.start_time.elapsed()
  }

  /// Gets the name of the effect.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Gets the metadata for the effect.
  pub fn metadata(&self) -> Vec<(String, String)> {
    self.metadata.lock().unwrap().clone()
  }
}

/// A trait for tracking effects.
pub trait EffectTracker {
  /// Called when an effect starts.
  fn on_start(&self, ctx: &TrackingContext);

  /// Called when an effect completes successfully.
  fn on_success(&self, ctx: &TrackingContext);

  /// Called when an effect fails.
  fn on_error(&self, ctx: &TrackingContext, error: &(dyn StdError + Send + Sync));
}

/// A handler that tracks effects.
pub struct TrackingHandler<H, T> {
  inner: H,
  tracker: T,
}

impl<H, T> TrackingHandler<H, T> {
  /// Creates a new tracking handler.
  pub fn new(inner: H, tracker: T) -> Self {
    Self { inner, tracker }
  }
}

impl<T, E, H, Tr> EffectHandler<T, E> for TrackingHandler<H, Tr>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
  H: EffectHandler<T, E>,
  Tr: EffectTracker + Send + Sync + 'static,
{
  fn handle(&self, ctx: HandlerContext<T, E>) -> Effect<T, E> {
    let tracking_ctx = TrackingContext::new("effect");
    self.tracker.on_start(&tracking_ctx);

    Effect::new(async move {
      let result = self.inner.handle(ctx).run().await;
      match &result {
        Ok(_) => self.tracker.on_success(&tracking_ctx),
        Err(e) => self.tracker.on_error(&tracking_ctx, e),
      }
      result
    })
  }
}

/// A basic effect tracker that logs to stdout.
pub struct LoggingTracker;

impl EffectTracker for LoggingTracker {
  fn on_start(&self, ctx: &TrackingContext) {
    println!("Starting effect: {}", ctx.name());
  }

  fn on_success(&self, ctx: &TrackingContext) {
    println!(
      "Effect {} completed successfully in {:?}",
      ctx.name(),
      ctx.duration()
    );
  }

  fn on_error(&self, ctx: &TrackingContext, error: &(dyn StdError + Send + Sync)) {
    println!(
      "Effect {} failed after {:?}: {}",
      ctx.name(),
      ctx.duration(),
      error
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};

  struct TestHandler;

  impl EffectHandler<i32, IoError> for TestHandler {
    fn handle(&self, ctx: HandlerContext<i32, IoError>) -> Effect<i32, IoError> {
      ctx.effect()
    }
  }

  #[tokio::test]
  async fn test_tracking_handler() {
    let inner = TestHandler;
    let tracker = LoggingTracker;
    let handler = TrackingHandler::new(inner, tracker);
    let effect = Effect::pure(42);
    let ctx = HandlerContext::new(effect);
    let result = handler.handle(ctx).run().await.unwrap();
    assert_eq!(result, 42);
  }

  #[tokio::test]
  async fn test_tracking_handler_error() {
    let inner = TestHandler;
    let tracker = LoggingTracker;
    let handler = TrackingHandler::new(inner, tracker);
    let effect = Effect::new(async { Err(IoError::new(ErrorKind::Other, "test error")) });
    let ctx = HandlerContext::new(effect);
    assert!(handler.handle(ctx).run().await.is_err());
  }

  #[tokio::test]
  async fn test_tracking_context_metadata() {
    let ctx = TrackingContext::new("test");
    ctx.add_metadata("key", "value");
    ctx.add_metadata("another", "data");

    let metadata = ctx.metadata();
    assert_eq!(metadata.len(), 2);
    assert_eq!(metadata[0], ("key".to_string(), "value".to_string()));
    assert_eq!(metadata[1], ("another".to_string(), "data".to_string()));
  }

  #[tokio::test]
  async fn test_tracking_context_duration() {
    let ctx = TrackingContext::new("test");
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    assert!(ctx.duration() >= std::time::Duration::from_millis(10));
  }
}
