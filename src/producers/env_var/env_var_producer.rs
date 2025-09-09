use crate::error::ErrorStrategy;
use crate::producer::ProducerConfig;

#[derive(Debug, Clone)]
pub struct EnvVarProducer {
  pub filter: Option<Vec<String>>,
  pub config: ProducerConfig<(String, String)>,
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
