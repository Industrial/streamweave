use super::process_consumer::ProcessConsumer;
use async_trait::async_trait;
use futures::StreamExt;
use std::process::Stdio;
use streamweave::{Consumer, ConsumerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::warn;

#[async_trait]
impl Consumer for ProcessConsumer {
  type InputPorts = (String,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let command = self.command.clone();
    let args = self.args.clone();
    let component_name = self.config.name.clone();

    let mut child = match Command::new(&command)
      .args(&args)
      .stdin(Stdio::piped())
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .spawn()
    {
      Ok(child) => child,
      Err(e) => {
        warn!(
          component = %component_name,
          command = %command,
          error = %e,
          "Failed to spawn process, all items will be dropped"
        );
        return;
      }
    };

    if let Some(mut stdin) = child.stdin.take() {
      while let Some(value) = stream.next().await {
        if let Err(e) = stdin.write_all(value.as_bytes()).await {
          warn!(
            component = %component_name,
            command = %command,
            error = %e,
            "Failed to write to process stdin"
          );
          break;
        }
        if let Err(e) = stdin.write_all(b"\n").await {
          warn!(
            component = %component_name,
            command = %command,
            error = %e,
            "Failed to write newline to process stdin"
          );
          break;
        }
      }
      if let Err(e) = stdin.flush().await {
        warn!(
          component = %component_name,
          command = %command,
          error = %e,
          "Failed to flush process stdin"
        );
      }
    }

    // Wait for process to complete
    let _ = child.wait().await;
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy {
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
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
