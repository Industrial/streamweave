use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Consumer, ConsumerConfig, Input};
use async_trait::async_trait;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;

/// A consumer that writes items to standard error (stderr).
///
/// This consumer writes each item from the stream to stderr, one per line.
/// It's useful for building command-line tools that output errors or diagnostics to stderr.
///
/// # Example
///
/// ```rust,no_run
/// use crate::consumers::StdioStderrConsumer;
/// use crate::Pipeline;
///
/// let consumer = StdioStderrConsumer::new();
/// let pipeline = Pipeline::new()
///     .producer(/* ... */)
///     .transformer(/* ... */)
///     .consumer(consumer);
/// ```
pub struct StdioStderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<T>,
}

impl<T> StdioStderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  /// Creates a new `StdioStderrConsumer` with default configuration.
  pub fn new() -> Self {
    Self {
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

impl<T> Default for StdioStderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

// Trait implementations for StdioStderrConsumer

impl<T> Input for StdioStderrConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type Input = T;
  type InputStream = futures::stream::BoxStream<'static, T>;
}

#[async_trait]
impl<T> Consumer for StdioStderrConsumer<T>
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
      component_name: if self.config.name.is_empty() {
        "stdio_stderr_consumer".to_string()
      } else {
        self.config.name.clone()
      },
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
