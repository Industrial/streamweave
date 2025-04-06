use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  error::Error,
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, stream};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;

pub struct EnvVarProducer {
  filter: Option<Vec<String>>,
  config: ProducerConfig,
}

impl EnvVarProducer {
  pub fn new() -> Self {
    Self {
      filter: None,
      config: ProducerConfig::default(),
    }
  }

  pub fn with_vars(vars: Vec<String>) -> Self {
    Self {
      filter: Some(vars),
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    self.config_mut().set_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config_mut().set_name(name);
    self
  }
}

impl Default for EnvVarProducer {
  fn default() -> Self {
    Self::new()
  }
}

impl Error for EnvVarProducer {
  type Error = StreamError;
}

impl Output for EnvVarProducer {
  type Output = (String, String);
  type OutputStream = Pin<Box<dyn Stream<Item = Result<(String, String), StreamError>> + Send>>;
}

impl Producer for EnvVarProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let config = self.config.clone();
    let vars = match &self.filter {
      Some(filter) => {
        let vars: Vec<_> = filter
          .iter()
          .map(|key| {
            std::env::var(key)
              .map(|value| (key.clone(), value))
              .map_err(|e| {
                StreamError::new(
                  Box::new(e),
                  self.create_error_context(None),
                  self.component_info(),
                )
              })
          })
          .collect();
        Box::pin(stream::iter(vars)) as Pin<Box<dyn Stream<Item = _> + Send>>
      }
      None => {
        let vars: Vec<_> = std::env::vars().map(Ok).collect();
        Box::pin(stream::iter(vars))
      }
    };
    vars
  }

  fn config(&self) -> &ProducerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut ProducerConfig {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError) -> ErrorAction {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Producer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "env_var_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use std::collections::HashMap;
  use std::env;
  use std::sync::Arc;

  struct TestEnvVarProducer {
    filter: Option<Vec<String>>,
    env_provider: Arc<dyn Fn(&str) -> Option<String> + Send + Sync>,
    config: ProducerConfig,
  }

  impl TestEnvVarProducer {
    fn new() -> Self {
      Self {
        filter: None,
        env_provider: Arc::new(|key| std::env::var(key).ok()),
        config: ProducerConfig::default(),
      }
    }

    fn with_vars(vars: Vec<String>) -> Self {
      Self {
        filter: Some(vars),
        env_provider: Arc::new(|key| std::env::var(key).ok()),
        config: ProducerConfig::default(),
      }
    }

    fn with_mock_env(env_vars: HashMap<String, String>) -> Self {
      let env_vars = Arc::new(env_vars);
      Self {
        filter: None,
        env_provider: Arc::new(move |key| env_vars.get(key).cloned()),
        config: ProducerConfig::default(),
      }
    }
  }

  impl Error for TestEnvVarProducer {
    type Error = StreamError;
  }

  impl Output for TestEnvVarProducer {
    type Output = (String, String);
    type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
  }

  impl Producer for TestEnvVarProducer {
    fn produce(&mut self) -> Self::OutputStream {
      let env_provider = self.env_provider.clone();
      let config = self.config.clone();
      let vars = match &self.filter {
        Some(filter) => {
          let vars: Vec<_> = filter
            .iter()
            .map(|key| {
              env_provider(key)
                .map(|value| (key.clone(), value))
                .ok_or_else(|| {
                  StreamError::new(
                    Box::new(std::env::VarError::NotPresent),
                    self.create_error_context(None),
                    self.component_info(),
                  )
                })
            })
            .collect();
          Box::pin(stream::iter(vars)) as Pin<Box<dyn Stream<Item = _> + Send>>
        }
        None => {
          let vars: Vec<_> = std::env::vars()
            .filter(|(key, _)| env_provider(key).is_some())
            .map(Ok)
            .collect();
          Box::pin(stream::iter(vars))
        }
      };
      vars
    }

    fn config(&self) -> &ProducerConfig {
      &self.config
    }

    fn config_mut(&mut self) -> &mut ProducerConfig {
      &mut self.config
    }

    fn handle_error(&self, error: &StreamError) -> ErrorAction {
      match self.config().error_strategy() {
        ErrorStrategy::Stop => ErrorAction::Stop,
        ErrorStrategy::Skip => ErrorAction::Skip,
        ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
        _ => ErrorAction::Stop,
      }
    }

    fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
      ErrorContext {
        timestamp: chrono::Utc::now(),
        item,
        stage: PipelineStage::Producer,
      }
    }

    fn component_info(&self) -> ComponentInfo {
      ComponentInfo {
        name: self
          .config()
          .name()
          .unwrap_or_else(|| "test_env_var_producer".to_string()),
        type_name: std::any::type_name::<Self>().to_string(),
      }
    }
  }

  #[tokio::test]
  async fn test_env_var_producer_all() {
    unsafe {
      env::set_var("TEST_VAR_1", "value1");
      env::set_var("TEST_VAR_2", "value2");
      env::set_var("TEST_VAR_3", "value3");
    }

    let mut producer = EnvVarProducer::new();
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.map(|r| r.unwrap()).collect().await;

    assert!(
      !result.is_empty(),
      "Expected environment variables, but found none"
    );
    assert!(
      result
        .iter()
        .any(|(k, v)| k == "TEST_VAR_1" && v == "value1")
    );
    assert!(
      result
        .iter()
        .any(|(k, v)| k == "TEST_VAR_2" && v == "value2")
    );
    assert!(
      result
        .iter()
        .any(|(k, v)| k == "TEST_VAR_3" && v == "value3")
    );

    unsafe {
      env::remove_var("TEST_VAR_1");
      env::remove_var("TEST_VAR_2");
      env::remove_var("TEST_VAR_3");
    }
  }

  #[tokio::test]
  async fn test_env_var_producer_filtered() {
    unsafe {
      env::set_var("TEST_FILTER_1", "value1");
      env::set_var("TEST_FILTER_2", "value2");
    }

    let mut producer = EnvVarProducer::with_vars(vec!["TEST_FILTER_1".to_string()]);
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result.len(), 1);
    assert_eq!(
      result[0],
      ("TEST_FILTER_1".to_string(), "value1".to_string())
    );

    unsafe {
      env::remove_var("TEST_FILTER_1");
      env::remove_var("TEST_FILTER_2");
    }
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut producer = EnvVarProducer::new()
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError::new(
      Box::new(std::env::VarError::NotPresent),
      producer.create_error_context(None),
      producer.component_info(),
    );

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
