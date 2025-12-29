use super::interval_producer::IntervalProducer;
use streamweave::{Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::time;

impl Producer for IntervalProducer {
  type OutputPorts = (std::time::SystemTime,);

  fn produce(&mut self) -> Self::OutputStream {
    let interval = self.interval;
    Box::pin(async_stream::stream! {
      let mut interval_stream = time::interval(interval);
      loop {
        interval_stream.tick().await;
        yield std::time::SystemTime::now();
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<std::time::SystemTime>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<std::time::SystemTime> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<std::time::SystemTime> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<std::time::SystemTime>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(
    &self,
    item: Option<std::time::SystemTime>,
  ) -> ErrorContext<std::time::SystemTime> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "interval_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "interval_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
