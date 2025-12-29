use super::directory_consumer::DirectoryConsumer;
use async_trait::async_trait;
use futures::StreamExt;
use streamweave::{Consumer, ConsumerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::fs;
use tracing::warn;

#[async_trait]
impl Consumer for DirectoryConsumer {
  type InputPorts = (String,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let component_name = self.config.name.clone();

    while let Some(path) = stream.next().await {
      if let Err(e) = fs::create_dir_all(&path).await {
        warn!(
          component = %component_name,
          path = %path,
          error = %e,
          "Failed to create directory"
        );
      }
    }
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
