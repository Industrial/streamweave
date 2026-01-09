use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Output, Producer, ProducerConfig};
use async_trait::async_trait;
use futures::{Stream, stream};
use std::pin::Pin;

/// A producer that yields environment variables as key-value pairs.
///
/// This producer reads environment variables and emits them as `(String, String)`
/// tuples. Optionally, it can filter to only specific variable names.
#[derive(Debug, Clone)]
pub struct EnvVarProducer {
  /// Optional filter to only include specific environment variable names.
  pub filter: Option<Vec<String>>,
  /// Configuration for the producer, including error handling strategy.
  pub config: ProducerConfig<(String, String)>,
}

impl EnvVarProducer {
  /// Creates a new `EnvVarProducer` that produces all environment variables.
  pub fn new() -> Self {
    Self {
      filter: None,
      config: ProducerConfig::default(),
    }
  }

  /// Creates a new `EnvVarProducer` that only produces the specified environment variables.
  ///
  /// # Arguments
  ///
  /// * `vars` - The list of environment variable names to include.
  pub fn with_vars(vars: Vec<String>) -> Self {
    Self {
      filter: Some(vars),
      config: ProducerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this producer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(String, String)>) -> Self {
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

impl Default for EnvVarProducer {
  fn default() -> Self {
    Self::new()
  }
}

impl Output for EnvVarProducer {
  type Output = (String, String);
  type OutputStream = Pin<Box<dyn Stream<Item = (String, String)> + Send>>;
}

#[async_trait]
impl Producer for EnvVarProducer {
  type OutputPorts = ((String, String),);

  fn produce(&mut self) -> Self::OutputStream {
    let _config = self.config.clone();

    (match &self.filter {
      Some(filter) => {
        let vars: Vec<_> = filter
          .iter()
          .filter_map(|key| std::env::var(key).map(|value| (key.clone(), value)).ok())
          .collect();
        Box::pin(stream::iter(vars)) as Pin<Box<dyn Stream<Item = _> + Send>>
      }
      None => {
        let vars: Vec<_> = std::env::vars().collect();
        Box::pin(stream::iter(vars))
      }
    }) as _
  }

  fn set_config_impl(&mut self, config: ProducerConfig<(String, String)>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<(String, String)> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<(String, String)> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<(String, String)>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<(String, String)>) -> ErrorContext<(String, String)> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "env_var_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "env_var_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
