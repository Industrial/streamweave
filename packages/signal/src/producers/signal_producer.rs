use streamweave::ProducerConfig;
use streamweave_error::ErrorStrategy;

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
