use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, stream};
use std::pin::Pin;

pub struct EnvVarProducer {
  filter: Option<Vec<String>>,
  config: ProducerConfig<(String, String)>,
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

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(String, String)>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

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

impl Producer for EnvVarProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let config = self.config.clone();
    let vars = match &self.filter {
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
    };
    vars
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
    match self.config.error_strategy() {
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
      name: self
        .config
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
    config: ProducerConfig<(String, String)>,
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

  impl Output for TestEnvVarProducer {
    type Output = (String, String);
    type OutputStream = Pin<Box<dyn Stream<Item = (String, String)> + Send>>;
  }

  impl Producer for TestEnvVarProducer {
    fn produce(&mut self) -> Self::OutputStream {
      let env_provider = self.env_provider.clone();
      let config = self.config.clone();
      let vars = match &self.filter {
        Some(filter) => {
          let vars: Vec<_> = filter
            .iter()
            .filter_map(|key| env_provider(key).map(|value| (key.clone(), value)))
            .collect();
          Box::pin(stream::iter(vars)) as Pin<Box<dyn Stream<Item = _> + Send>>
        }
        None => {
          let vars: Vec<_> = std::env::vars()
            .filter(|(key, _)| env_provider(key).is_some())
            .collect();
          Box::pin(stream::iter(vars))
        }
      };
      vars
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
      match self.config.error_strategy() {
        ErrorStrategy::Stop => ErrorAction::Stop,
        ErrorStrategy::Skip => ErrorAction::Skip,
        ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
        _ => ErrorAction::Stop,
      }
    }

    fn create_error_context(
      &self,
      item: Option<(String, String)>,
    ) -> ErrorContext<(String, String)> {
      ErrorContext {
        timestamp: chrono::Utc::now(),
        item,
        stage: PipelineStage::Producer,
      }
    }

    fn component_info(&self) -> ComponentInfo {
      ComponentInfo {
        name: self
          .config
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
    let result: Vec<(String, String)> = stream.collect().await;

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
      env::set_var("TEST_VAR_1", "value1");
      env::set_var("TEST_VAR_2", "value2");
    }

    let mut producer = EnvVarProducer::with_vars(vec!["TEST_VAR_1".to_string()]);
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.collect().await;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], ("TEST_VAR_1".to_string(), "value1".to_string()));

    unsafe {
      env::remove_var("TEST_VAR_1");
      env::remove_var("TEST_VAR_2");
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

    let error = StreamError {
      source: Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "test error",
      )),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "EnvVarProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
