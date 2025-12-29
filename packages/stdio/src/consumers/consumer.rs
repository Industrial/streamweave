use super::{stderr_consumer::StderrConsumer, stdout_consumer::StdoutConsumer};
use async_trait::async_trait;
use futures::StreamExt;
use streamweave::{Consumer, ConsumerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::io::AsyncWriteExt;

#[async_trait]
impl<T> Consumer for StdoutConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let mut stdout = tokio::io::stdout();
    let component_name = self.config.name.clone();

    while let Some(value) = stream.next().await {
      let output = format!("{}\n", value);
      match stdout.write_all(output.as_bytes()).await {
        Ok(_) => {}
        Err(e) => {
          tracing::warn!(
            component = %component_name,
            error = %e,
            "Failed to write to stdout, continuing"
          );
        }
      }
    }

    // Flush stdout to ensure all output is written
    if let Err(e) = stdout.flush().await {
      tracing::warn!(
        component = %component_name,
        error = %e,
        "Failed to flush stdout"
      );
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
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

#[async_trait]
impl<T> Consumer for StderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let mut stderr = tokio::io::stderr();
    let component_name = self.config.name.clone();

    while let Some(value) = stream.next().await {
      let output = format!("{}\n", value);
      match stderr.write_all(output.as_bytes()).await {
        Ok(_) => {}
        Err(e) => {
          tracing::warn!(
            component = %component_name,
            error = %e,
            "Failed to write to stderr, continuing"
          );
        }
      }
    }

    // Flush stderr to ensure all output is written
    if let Err(e) = stderr.flush().await {
      tracing::warn!(
        component = %component_name,
        error = %e,
        "Failed to flush stderr"
      );
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
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
