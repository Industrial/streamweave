//! # Trace Transformer
//!
//! Transformer that traces and logs items as they pass through the pipeline
//! without modifying them. This is useful for debugging and monitoring pipeline
//! behavior, allowing you to see what data is flowing through at specific points.
//!
//! ## Overview
//!
//! The Trace Transformer provides:
//!
//! - **Debug Logging**: Logs each item that passes through at configurable log levels
//! - **Pass-Through**: Items are emitted unchanged (no transformation)
//! - **Log Level Control**: Configurable trace or debug logging levels
//! - **Type Generic**: Works with any `Send + Sync + Clone` type
//! - **Pipeline Debugging**: Essential tool for debugging complex pipelines
//!
//! ## Input/Output
//!
//! - **Input**: `Message<T>` - Items to trace
//! - **Output**: `Message<T>` - The same items (unchanged, but logged)
//!
//! ## Log Levels
//!
//! - **Trace**: Most verbose logging (trace level)
//! - **Debug**: Standard debug logging (debug level, default)
//!
//! ## Example
//!
//! ```rust
//! use streamweave::transformers::TraceTransformer;
//!
//! let transformer = TraceTransformer::new();
//! // Items will be logged at debug level and passed through unchanged
//! ```

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tracing::{debug, trace};

/// A transformer that traces/logs items as they pass through without modifying them.
///
/// This transformer is useful for debugging pipelines. It logs each item that
/// passes through and emits it unchanged. The logging level can be controlled
/// via the `log_level` parameter.
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::TraceTransformer;
///
/// let transformer = TraceTransformer::new();
/// // Items will be logged at debug level and passed through unchanged
/// ```
pub struct TraceTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Logging level for tracing
  log_level: TraceLevel,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
}

/// Logging level for trace transformer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceLevel {
  /// Trace level (most verbose)
  Trace,
  /// Debug level (default)
  #[default]
  Debug,
}

impl<T> TraceTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `TraceTransformer` with default debug logging.
  pub fn new() -> Self {
    Self {
      log_level: TraceLevel::Debug,
      config: TransformerConfig::default(),
    }
  }

  /// Creates a new `TraceTransformer` with the specified log level.
  ///
  /// # Arguments
  ///
  /// * `log_level` - The logging level to use for tracing.
  pub fn with_log_level(log_level: TraceLevel) -> Self {
    Self {
      log_level,
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Default for TraceTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> Clone for TraceTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      log_level: self.log_level,
      config: self.config.clone(),
    }
  }
}

impl<T> Input for TraceTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for TraceTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for TraceTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let log_level = self.log_level;
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "trace_transformer".to_string());

    Box::pin(input.map(move |item| {
      match log_level {
        TraceLevel::Trace => {
          trace!(
            component = %component_name,
            item = ?item,
            "Tracing item through transformer"
          );
        }
        TraceLevel::Debug => {
          debug!(
            component = %component_name,
            item = ?item,
            "Tracing item through transformer"
          );
        }
      }
      item
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "trace_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
