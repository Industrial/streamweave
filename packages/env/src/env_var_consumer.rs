use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use streamweave::{Consumer, ConsumerConfig, Input};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tracing::{error, warn};

/// A consumer that sets environment variables from key-value pairs.
///
/// This consumer takes `(String, String)` tuples from the stream and sets them
/// as environment variables. It's useful for configuring the runtime environment
/// based on stream data.
#[derive(Debug, Clone)]
pub struct EnvVarConsumer {
  /// Configuration for the consumer, including error handling strategy.
  pub config: ConsumerConfig<(String, String)>,
}

impl EnvVarConsumer {
  /// Creates a new `EnvVarConsumer`.
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
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(String, String)>) -> Self {
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

  /// Validates if a string is a valid environment variable name.
  ///
  /// Environment variable names typically:
  /// - Must not be empty
  /// - Should start with a letter or underscore
  /// - Can contain letters, digits, and underscores
  /// - Should not contain special characters or spaces
  fn is_valid_env_var_name(name: &str) -> bool {
    if name.is_empty() {
      return false;
    }

    // First character must be a letter or underscore
    let first_char = name.chars().next().unwrap();
    if !first_char.is_alphabetic() && first_char != '_' {
      return false;
    }

    // All characters must be alphanumeric or underscore
    name.chars().all(|c| c.is_alphanumeric() || c == '_')
  }
}

impl Default for EnvVarConsumer {
  fn default() -> Self {
    Self::new()
  }
}

impl Input for EnvVarConsumer {
  type Input = (String, String);
  type InputStream = Pin<Box<dyn Stream<Item = (String, String)> + Send>>;
}

#[async_trait]
impl Consumer for EnvVarConsumer {
  type InputPorts = ((String, String),);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some((key, value)) = stream.next().await {
      // Validate the key before setting
      if Self::is_valid_env_var_name(&key) {
        unsafe {
          std::env::set_var(&key, &value);
        }
      } else {
        let error_msg = format!("Invalid environment variable name: {}", key);
        warn!("{}", error_msg);

        // Create error context for invalid variable names
        let error_context = self.create_error_context(Some((key.clone(), value)));
        let stream_error = StreamError {
          source: Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            error_msg,
          )),
          context: error_context,
          component: self.component_info(),
          retries: 0,
        };

        match self.handle_error(&stream_error) {
          ErrorAction::Stop => {
            error!("Stopping due to invalid environment variable name: {}", key);
            break;
          }
          ErrorAction::Skip => {
            warn!("Skipping invalid environment variable: {}", key);
            continue;
          }
          ErrorAction::Retry => {
            warn!("Retrying invalid environment variable: {}", key);
            // For retry, we'll just skip since we can't retry a validation error
            continue;
          }
        }
      }
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<(String, String)>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<(String, String)> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<(String, String)> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<(String, String)>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<(String, String)>) -> ErrorContext<(String, String)> {
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
