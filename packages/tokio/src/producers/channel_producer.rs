use streamweave::ProducerConfig;
use streamweave_error::ErrorStrategy;
use tokio::sync::mpsc::Receiver;

/// A producer that reads items from a `tokio::sync::mpsc::Receiver`.
///
/// This producer reads items from a tokio channel receiver and produces them
/// into a StreamWeave stream. It's useful for integrating StreamWeave with
/// existing async code that uses channels.
pub struct ChannelProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The channel receiver to read items from.
  pub receiver: Option<Receiver<T>>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<T>,
}

impl<T> ChannelProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `ChannelProducer` with the given receiver.
  ///
  /// # Arguments
  ///
  /// * `receiver` - The `tokio::sync::mpsc::Receiver` to read items from.
  pub fn new(receiver: Receiver<T>) -> Self {
    Self {
      receiver: Some(receiver),
      config: ProducerConfig::default(),
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
