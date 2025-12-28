use super::channel_producer::ChannelProducer;
use streamweave::{Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio_stream::wrappers::ReceiverStream;

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
