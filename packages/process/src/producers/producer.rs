use super::process_producer::ProcessProducer;
use std::process::Stdio;
use streamweave::{Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

impl Producer for ProcessProducer {
  type OutputPorts = (String,);

  fn produce(&mut self) -> Self::OutputStream {
    let command = self.command.clone();
    let args = self.args.clone();

    Box::pin(async_stream::stream! {
      let child = match Command::new(&command)
          .args(&args)
          .stdout(Stdio::piped())
          .spawn() {
          Ok(child) => child,
          Err(_) => return,
      };

      let stdout = match child.stdout {
          Some(stdout) => stdout,
          None => return,
      };

      let reader = BufReader::new(stdout);
      let mut lines = reader.lines();

      while let Some(line) = lines.next_line().await.ok().flatten() {
          yield line;
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
        .unwrap_or_else(|| "process_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "process_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
