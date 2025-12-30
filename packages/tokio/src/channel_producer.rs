use futures::Stream;
use std::pin::Pin;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;

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

impl<T> Output for ChannelProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Producer for ChannelProducer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type OutputPorts = (T,);

  fn produce(&mut self) -> Self::OutputStream {
    // Take ownership of the receiver
    let receiver = self.receiver.take().expect("Receiver already consumed");

    // Convert tokio Receiver to a Stream
    let stream = ReceiverStream::new(receiver);

    Box::pin(stream)
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
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
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "channel_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "channel_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
