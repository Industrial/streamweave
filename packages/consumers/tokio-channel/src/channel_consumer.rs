use streamweave::ConsumerConfig;
use streamweave_error::ErrorStrategy;
use tokio::sync::mpsc::Sender;

/// A consumer that sends items to a `tokio::sync::mpsc::Sender`.
///
/// This consumer forwards all items from the stream to the channel sender.
/// It's useful for integrating StreamWeave with existing async code that uses channels.
pub struct ChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The channel sender to forward items to.
  pub channel: Option<Sender<T>>,
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T> ChannelConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `ChannelConsumer` with the given sender.
  ///
  /// # Arguments
  ///
  /// * `sender` - The `tokio::sync::mpsc::Sender` to send items to.
  pub fn new(sender: Sender<T>) -> Self {
    Self {
      channel: Some(sender),
      config: ConsumerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this consumer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this consumer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this consumer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }
}
