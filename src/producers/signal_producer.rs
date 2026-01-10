use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Represents a Unix signal that was received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
  /// SIGINT - Interrupt signal
  Interrupt,
  /// SIGTERM - Termination signal
  Terminate,
}

impl Signal {
  /// Convert from a signal number
  pub fn from_number(sig: i32) -> Option<Self> {
    match sig {
      2 => Some(Signal::Interrupt),  // SIGINT
      15 => Some(Signal::Terminate), // SIGTERM
      _ => None,
    }
  }

  /// Get the signal number
  pub fn number(&self) -> i32 {
    match self {
      Signal::Interrupt => 2,
      Signal::Terminate => 15,
    }
  }
}

/// A producer that emits events when Unix signals are received.
#[derive(Clone)]
pub struct SignalProducer {
  /// Configuration for the producer
  pub config: ProducerConfig<Signal>,
}

impl SignalProducer {
  /// Creates a new `SignalProducer`.
  pub fn new() -> Self {
    Self {
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Signal>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for SignalProducer {
  fn default() -> Self {
    Self::new()
  }
}

impl Output for SignalProducer {
  type Output = Signal;
  type OutputStream = Pin<Box<dyn Stream<Item = Signal> + Send>>;
}

#[async_trait]
impl Producer for SignalProducer {
  type OutputPorts = (Signal,);

  fn produce(&mut self) -> Self::OutputStream {
    Box::pin(async_stream::stream! {
      // Create signal streams for common signals
      if let Ok(mut sigint) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        && let Ok(mut sigterm) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
      {
        loop {
          tokio::select! {
            _ = sigint.recv() => {
              yield Signal::Interrupt;
            }
            _ = sigterm.recv() => {
              yield Signal::Terminate;
            }
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<Signal>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<Signal> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Signal> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Signal>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Signal>) -> ErrorContext<Signal> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "signal_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "signal_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
