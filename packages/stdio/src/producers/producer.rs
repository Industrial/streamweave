use super::stdin_producer::StdinProducer;
use streamweave::{Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::io::{AsyncBufReadExt, BufReader};

impl Producer for StdinProducer {
  type OutputPorts = (String,);

  fn produce(&mut self) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "stdin_producer".to_string());

    Box::pin(async_stream::stream! {
      let stdin = tokio::io::stdin();
      let reader = BufReader::new(stdin);
      let mut lines = reader.lines();

      loop {
        match lines.next_line().await {
          Ok(Some(line)) => yield line,
          Ok(None) => break, // EOF
          Err(e) => {
            tracing::warn!(
              component = %component_name,
              error = %e,
              "Failed to read line from stdin, stopping"
            );
            break;
          }
        }
      }
    })
  }

  fn set_config_impl(&mut self, config: ProducerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "stdin_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "stdin_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
