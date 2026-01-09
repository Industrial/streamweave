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

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[test]
  fn test_signal_from_number() {
    assert_eq!(Signal::from_number(2), Some(Signal::Interrupt));
    assert_eq!(Signal::from_number(15), Some(Signal::Terminate));
    assert_eq!(Signal::from_number(99), None);
    assert_eq!(Signal::from_number(0), None);
    assert_eq!(Signal::from_number(-1), None);
  }

  #[test]
  fn test_signal_number() {
    assert_eq!(Signal::Interrupt.number(), 2);
    assert_eq!(Signal::Terminate.number(), 15);
  }

  #[test]
  fn test_signal_debug() {
    let signal = Signal::Interrupt;
    let debug_str = format!("{:?}", signal);
    assert!(debug_str.contains("Interrupt"));

    let signal = Signal::Terminate;
    let debug_str = format!("{:?}", signal);
    assert!(debug_str.contains("Terminate"));
  }

  #[test]
  fn test_signal_clone() {
    let signal1 = Signal::Interrupt;
    let signal2 = signal1;
    assert_eq!(signal1, signal2);

    let signal1 = Signal::Terminate;
    let signal2 = signal1;
    assert_eq!(signal1, signal2);
  }

  #[test]
  fn test_signal_copy() {
    let signal1 = Signal::Interrupt;
    let signal2 = signal1; // Copy, not move
    assert_eq!(signal1, signal2);
  }

  #[test]
  fn test_signal_eq() {
    assert_eq!(Signal::Interrupt, Signal::Interrupt);
    assert_eq!(Signal::Terminate, Signal::Terminate);
    assert_ne!(Signal::Interrupt, Signal::Terminate);
  }

  #[test]
  fn test_signal_producer_new() {
    let producer = SignalProducer::new();
    assert_eq!(producer.config.name, None);
  }

  #[test]
  fn test_signal_producer_default() {
    let producer = SignalProducer::default();
    assert_eq!(producer.config.name, None);
  }

  #[test]
  fn test_signal_producer_with_name() {
    let producer = SignalProducer::new().with_name("test_producer".to_string());
    assert_eq!(producer.config.name, Some("test_producer".to_string()));
  }

  #[test]
  fn test_signal_producer_with_error_strategy() {
    let producer = SignalProducer::new().with_error_strategy(ErrorStrategy::<Signal>::Skip);
    assert_eq!(
      producer.config.error_strategy(),
      ErrorStrategy::<Signal>::Skip
    );
  }

  #[test]
  fn test_signal_producer_set_config_impl() {
    let mut producer = SignalProducer::new();
    let config = ProducerConfig {
      name: Some("test_config".to_string()),
      error_strategy: ErrorStrategy::Skip,
    };

    producer.set_config_impl(config.clone());

    assert_eq!(producer.config.name, Some("test_config".to_string()));
    assert_eq!(
      producer.config.error_strategy(),
      ErrorStrategy::<Signal>::Skip
    );
  }

  #[test]
  fn test_signal_producer_get_config_impl() {
    let producer = SignalProducer::new().with_name("test_producer".to_string());
    let config = producer.get_config_impl();

    assert_eq!(config.name(), Some("test_producer".to_string()));
  }

  #[test]
  fn test_signal_producer_get_config_mut_impl() {
    let mut producer = SignalProducer::new();
    let config_mut = producer.get_config_mut_impl();

    config_mut.name = Some("mutated_name".to_string());
    config_mut.error_strategy = ErrorStrategy::Retry(5);

    assert_eq!(producer.config.name, Some("mutated_name".to_string()));
    assert_eq!(
      producer.config.error_strategy(),
      ErrorStrategy::<Signal>::Retry(5)
    );
  }

  #[test]
  fn test_signal_producer_config() {
    let producer = SignalProducer::new().with_name("test_producer".to_string());
    let config = producer.config();

    assert_eq!(config.name(), Some("test_producer".to_string()));
  }

  #[test]
  fn test_signal_producer_config_mut() {
    let mut producer = SignalProducer::new();
    let config_mut = producer.config_mut();

    config_mut.name = Some("mutated_via_config_mut".to_string());
    config_mut.error_strategy = ErrorStrategy::Retry(10);

    assert_eq!(
      producer.config.name,
      Some("mutated_via_config_mut".to_string())
    );
    assert_eq!(
      producer.config.error_strategy(),
      ErrorStrategy::<Signal>::Retry(10)
    );
  }

  #[test]
  fn test_signal_producer_with_config() {
    let config = ProducerConfig {
      name: Some("config_name".to_string()),
      error_strategy: ErrorStrategy::Skip,
    };

    let producer = SignalProducer::new().with_config(config.clone());

    assert_eq!(producer.config.name, Some("config_name".to_string()));
    assert_eq!(
      producer.config.error_strategy(),
      ErrorStrategy::<Signal>::Skip
    );
  }

  #[test]
  fn test_signal_producer_error_handling_stop() {
    let producer = SignalProducer::new();
    let error = StreamError::new(
      Box::new(TestError("test error".to_string())),
      ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(Signal::Interrupt),
        component_name: "signal_producer".to_string(),
        component_type: std::any::type_name::<SignalProducer>().to_string(),
      },
      ComponentInfo {
        name: "signal_producer".to_string(),
        type_name: std::any::type_name::<SignalProducer>().to_string(),
      },
    );

    let action = producer.handle_error(&error);
    assert_eq!(action, ErrorAction::Stop);
  }

  #[test]
  fn test_signal_producer_error_handling_skip() {
    let producer = SignalProducer::new().with_error_strategy(ErrorStrategy::<Signal>::Skip);
    let error = StreamError::new(
      Box::new(TestError("test error".to_string())),
      ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(Signal::Interrupt),
        component_name: "signal_producer".to_string(),
        component_type: std::any::type_name::<SignalProducer>().to_string(),
      },
      ComponentInfo {
        name: "signal_producer".to_string(),
        type_name: std::any::type_name::<SignalProducer>().to_string(),
      },
    );

    let action = producer.handle_error(&error);
    assert_eq!(action, ErrorAction::Skip);
  }

  #[test]
  fn test_signal_producer_error_handling_retry() {
    let producer = SignalProducer::new().with_error_strategy(ErrorStrategy::<Signal>::Retry(3));
    let mut error = StreamError::new(
      Box::new(TestError("test error".to_string())),
      ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(Signal::Interrupt),
        component_name: "signal_producer".to_string(),
        component_type: std::any::type_name::<SignalProducer>().to_string(),
      },
      ComponentInfo {
        name: "signal_producer".to_string(),
        type_name: std::any::type_name::<SignalProducer>().to_string(),
      },
    );

    // First retry should succeed
    error.retries = 0;
    let action = producer.handle_error(&error);
    assert_eq!(action, ErrorAction::Retry);

    // After max retries, should stop
    error.retries = 3;
    let action = producer.handle_error(&error);
    assert_eq!(action, ErrorAction::Stop);
  }

  #[test]
  fn test_signal_producer_error_handling_retry_exceeded() {
    let producer = SignalProducer::new().with_error_strategy(ErrorStrategy::<Signal>::Retry(3));

    let mut error = StreamError::new(
      Box::new(TestError("test error".to_string())),
      ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(Signal::Interrupt),
        component_name: "signal_producer".to_string(),
        component_type: std::any::type_name::<SignalProducer>().to_string(),
      },
      ComponentInfo {
        name: "signal_producer".to_string(),
        type_name: std::any::type_name::<SignalProducer>().to_string(),
      },
    );

    // At max retries, should stop (not retry)
    error.retries = 3;
    let action = producer.handle_error(&error);
    assert_eq!(action, ErrorAction::Stop);

    // Beyond max retries, should stop
    error.retries = 4;
    let action = producer.handle_error(&error);
    assert_eq!(action, ErrorAction::Stop);
  }

  #[test]
  fn test_signal_producer_error_handling_custom() {
    let custom_handler = |error: &StreamError<Signal>| {
      if error.retries < 2 {
        ErrorAction::Retry
      } else {
        ErrorAction::Skip
      }
    };

    let producer =
      SignalProducer::new().with_error_strategy(ErrorStrategy::new_custom(custom_handler));

    let mut error = StreamError::new(
      Box::new(TestError("test error".to_string())),
      ErrorContext {
        timestamp: chrono::Utc::now(),
        item: Some(Signal::Interrupt),
        component_name: "signal_producer".to_string(),
        component_type: std::any::type_name::<SignalProducer>().to_string(),
      },
      ComponentInfo {
        name: "signal_producer".to_string(),
        type_name: std::any::type_name::<SignalProducer>().to_string(),
      },
    );

    // First retry should succeed
    error.retries = 0;
    let action = producer.handle_error(&error);
    assert_eq!(action, ErrorAction::Retry);

    // Second retry should succeed
    error.retries = 1;
    let action = producer.handle_error(&error);
    assert_eq!(action, ErrorAction::Retry);

    // After 2 retries, should skip
    error.retries = 2;
    let action = producer.handle_error(&error);
    assert_eq!(action, ErrorAction::Skip);
  }

  #[test]
  fn test_signal_producer_create_error_context() {
    let producer = SignalProducer::new().with_name("test_producer".to_string());
    let context = producer.create_error_context(Some(Signal::Interrupt));

    assert_eq!(context.item, Some(Signal::Interrupt));
    assert_eq!(context.component_name, "test_producer");
    assert_eq!(
      context.component_type,
      std::any::type_name::<SignalProducer>()
    );
  }

  #[test]
  fn test_signal_producer_create_error_context_none_item() {
    let producer = SignalProducer::new().with_name("test_producer".to_string());
    let context = producer.create_error_context(None);

    assert_eq!(context.item, None);
    assert_eq!(context.component_name, "test_producer");
    assert_eq!(
      context.component_type,
      std::any::type_name::<SignalProducer>()
    );
  }

  #[test]
  fn test_signal_producer_create_error_context_default_name() {
    let producer = SignalProducer::new();
    let context = producer.create_error_context(Some(Signal::Terminate));

    assert_eq!(context.item, Some(Signal::Terminate));
    assert_eq!(context.component_name, "signal_producer");
    assert_eq!(
      context.component_type,
      std::any::type_name::<SignalProducer>()
    );
  }

  #[test]
  fn test_signal_producer_component_info() {
    let producer = SignalProducer::new().with_name("test_producer".to_string());
    let info = producer.component_info();

    assert_eq!(info.name, "test_producer");
    assert_eq!(info.type_name, std::any::type_name::<SignalProducer>());
  }

  #[test]
  fn test_signal_producer_component_info_default() {
    let producer = SignalProducer::new();
    let info = producer.component_info();

    assert_eq!(info.name, "signal_producer");
    assert_eq!(info.type_name, std::any::type_name::<SignalProducer>());
  }

  #[test]
  fn test_signal_producer_output_trait() {
    let _producer = SignalProducer::new();
    // Verify Output trait is implemented
    let _output_type: <SignalProducer as Output>::Output = Signal::Interrupt;
    let _output_stream_type: <SignalProducer as Output>::OutputStream =
      Box::pin(futures::stream::empty());
  }

  #[test]
  fn test_signal_producer_output_ports() {
    let _producer = SignalProducer::new();
    // Verify OutputPorts is correctly set
    let _ports: <SignalProducer as Producer>::OutputPorts = (Signal::Interrupt,);
  }

  #[tokio::test]
  async fn test_signal_producer_produce() {
    let mut producer = SignalProducer::new();
    let stream = producer.produce();

    // The stream should be created successfully
    // Note: We can't easily test signal reception in unit tests without actually
    // sending signals, but we verify the stream is created and is a valid stream
    // Just verify it's a stream - we can't test actual signal reception
    let _ = stream;
  }
}
