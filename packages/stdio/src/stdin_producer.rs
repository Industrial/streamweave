use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use streamweave::{Output, Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tokio::io::{AsyncBufReadExt, BufReader};

/// A producer that reads lines from standard input (stdin).
///
/// This producer reads lines from stdin and emits each line as a string item.
/// It's useful for building command-line tools that process input from stdin.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave_stdio::StdinProducer;
/// use streamweave::Pipeline;
///
/// let producer = StdinProducer::new();
/// let pipeline = Pipeline::new()
///     .producer(producer)
///     .transformer(/* ... */)
///     .consumer(/* ... */);
/// ```
pub struct StdinProducer {
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<String>,
}

impl StdinProducer {
  /// Creates a new `StdinProducer` with default configuration.
  pub fn new() -> Self {
    Self {
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
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

impl Default for StdinProducer {
  fn default() -> Self {
    Self::new()
  }
}

// Trait implementations for StdinProducer

impl Output for StdinProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
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
