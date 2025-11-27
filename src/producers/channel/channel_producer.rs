use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;
use tokio::sync::mpsc;

/// A producer that emits items received from a `tokio::sync::mpsc::Receiver`.
///
/// This producer reads items from a channel receiver and emits them as a stream.
/// It's useful for integrating StreamWeave with existing async code that uses channels.
pub struct ChannelProducer<T: std::fmt::Debug + Clone + Send + Sync> {
  /// The receiver channel from which items will be read.
  pub rx: mpsc::Receiver<T>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync> ChannelProducer<T> {
  /// Creates a new `ChannelProducer` with the given receiver.
  ///
  /// # Arguments
  ///
  /// * `rx` - The `tokio::sync::mpsc::Receiver` to read items from.
  pub fn new(rx: mpsc::Receiver<T>) -> Self {
    Self {
      rx,
      config: crate::producer::ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this producer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this producer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
