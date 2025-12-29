use super::signal_producer::{Signal, SignalProducer};
use streamweave::{Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

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
