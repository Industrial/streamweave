use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::structs::producers::env_var::EnvVarProducer;
use crate::traits::producer::{Producer, ProducerConfig};
use futures::{Stream, stream};
use std::pin::Pin;

impl Producer for EnvVarProducer {
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
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
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
        .name()
        .unwrap_or_else(|| "env_var_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::output::Output;
  use futures::StreamExt;
  use proptest::prelude::*;
  use std::collections::HashMap;
  use std::env;
  use std::sync::{Arc, Mutex};

  // Global state for environment variable testing
  static ENV_LOCK: Mutex<()> = Mutex::new(());

  struct TestEnvVarProducer {
    filter: Option<Vec<String>>,
    env_vars: Option<Arc<HashMap<String, String>>>,
    config: ProducerConfig<(String, String)>,
  }

  impl TestEnvVarProducer {
    fn with_mock_env(env_vars: HashMap<String, String>) -> Self {
      Self {
        filter: None,
        env_vars: Some(Arc::new(env_vars)),
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
      let _config = self.config.clone();
      let vars = match &self.filter {
        Some(filter) => {
          let vars: Vec<_> = filter
            .iter()
            .filter_map(|key| {
              if let Some(env_vars) = &self.env_vars {
                env_vars.get(key).map(|value| (key.clone(), value.clone()))
              } else {
                std::env::var(key).map(|value| (key.clone(), value)).ok()
              }
            })
            .collect();
          Box::pin(stream::iter(vars)) as Pin<Box<dyn Stream<Item = _> + Send>>
        }
        None => {
          let vars: Vec<_> = if let Some(env_vars) = &self.env_vars {
            env_vars
              .iter()
              .map(|(k, v)| (k.clone(), v.clone()))
              .collect()
          } else {
            std::env::vars().collect()
          };
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
        component_name: "test".to_string(),
        component_type: "TestEnvVarProducer".to_string(),
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
    let mut env_vars = HashMap::new();
    env_vars.insert("TEST_VAR_1".to_string(), "value1".to_string());
    env_vars.insert("TEST_VAR_2".to_string(), "value2".to_string());
    env_vars.insert("TEST_VAR_3".to_string(), "value3".to_string());

    let mut producer = TestEnvVarProducer::with_mock_env(env_vars);
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
  }

  #[tokio::test]
  async fn test_env_var_producer_filtered() {
    let _lock = ENV_LOCK.lock().unwrap();

    unsafe {
      env::set_var("TEST_VAR_1", "value1");
      env::set_var("TEST_VAR_2", "value2");
      env::set_var("TEST_VAR_3", "value3");
    }

    let mut producer = EnvVarProducer::with_vars(vec!["TEST_VAR_1".to_string()]);
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.collect().await;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], ("TEST_VAR_1".to_string(), "value1".to_string()));

    unsafe {
      env::remove_var("TEST_VAR_1");
      env::remove_var("TEST_VAR_2");
      env::remove_var("TEST_VAR_3");
    }
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let producer = EnvVarProducer::new()
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
        component_name: "test".to_string(),
        component_type: "TestEnvVarProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestEnvVarProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }

  #[tokio::test]
  async fn test_env_var_producer_new() {
    let producer = EnvVarProducer::new();
    assert_eq!(producer.filter, None);
    assert_eq!(producer.config.error_strategy(), ErrorStrategy::Stop);
    assert_eq!(producer.config.name(), None);
  }

  #[tokio::test]
  async fn test_env_var_producer_with_vars() {
    let vars = vec!["VAR1".to_string(), "VAR2".to_string()];
    let producer = EnvVarProducer::with_vars(vars.clone());
    assert_eq!(producer.filter, Some(vars));
    assert_eq!(producer.config.error_strategy(), ErrorStrategy::Stop);
    assert_eq!(producer.config.name(), None);
  }

  #[tokio::test]
  async fn test_env_var_producer_default() {
    let producer = EnvVarProducer::default();
    assert_eq!(producer.filter, None);
    assert_eq!(producer.config.error_strategy(), ErrorStrategy::Stop);
    assert_eq!(producer.config.name(), None);
  }

  #[tokio::test]
  async fn test_env_var_producer_with_error_strategy() {
    let producer = EnvVarProducer::new().with_error_strategy(ErrorStrategy::Retry(3));

    assert_eq!(producer.config.error_strategy(), ErrorStrategy::Retry(3));

    // Test error handling with retry strategy
    let error = StreamError {
      source: Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "test error",
      )),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestEnvVarProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestEnvVarProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Retry);

    // Test retry exhaustion
    let error_exhausted = StreamError {
      source: Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "test error",
      )),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestEnvVarProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestEnvVarProducer".to_string(),
      },
      retries: 3,
    };

    assert_eq!(producer.handle_error(&error_exhausted), ErrorAction::Stop);
  }

  #[tokio::test]
  async fn test_env_var_producer_with_name() {
    let producer = EnvVarProducer::new().with_name("custom_name".to_string());
    assert_eq!(producer.config.name(), Some("custom_name".to_string()));
  }

  #[tokio::test]
  async fn test_env_var_producer_config_mut() {
    let mut producer = EnvVarProducer::new();
    let config = producer.config_mut();
    config.name = Some("mutated_name".to_string());
    assert_eq!(producer.config.name(), Some("mutated_name".to_string()));
  }

  #[tokio::test]
  async fn test_env_var_producer_set_config() {
    let mut producer = EnvVarProducer::new();
    let new_config = ProducerConfig::default()
      .with_name("new_name".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    producer.set_config(new_config);
    assert_eq!(producer.config.name(), Some("new_name".to_string()));
    assert_eq!(producer.config.error_strategy(), ErrorStrategy::Skip);
  }

  #[tokio::test]
  async fn test_env_var_producer_create_error_context() {
    let producer = EnvVarProducer::new().with_name("test_producer".to_string());
    let context = producer.create_error_context(Some(("key".to_string(), "value".to_string())));

    assert_eq!(context.component_name, "test_producer");
    assert_eq!(
      context.component_type,
      std::any::type_name::<EnvVarProducer>()
    );
    assert_eq!(context.item, Some(("key".to_string(), "value".to_string())));

    // Test with no item
    let context_no_item = producer.create_error_context(None);
    assert_eq!(context_no_item.item, None);
  }

  #[tokio::test]
  async fn test_env_var_producer_component_info() {
    let producer = EnvVarProducer::new().with_name("test_producer".to_string());
    let info = producer.component_info();

    assert_eq!(info.name, "test_producer");
    assert_eq!(info.type_name, std::any::type_name::<EnvVarProducer>());

    // Test default name
    let producer_default = EnvVarProducer::new();
    let info_default = producer_default.component_info();
    assert_eq!(info_default.name, "env_var_producer");
  }

  #[tokio::test]
  async fn test_env_var_producer_empty_filter() {
    let producer = EnvVarProducer::with_vars(vec![]);
    let mut producer = producer;
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.collect().await;
    assert_eq!(result.len(), 0);
  }

  #[tokio::test]
  async fn test_env_var_producer_nonexistent_vars() {
    let _lock = ENV_LOCK.lock().unwrap();

    let producer = EnvVarProducer::with_vars(vec!["NONEXISTENT_VAR".to_string()]);
    let mut producer = producer;
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.collect().await;
    assert_eq!(result.len(), 0);
  }

  #[tokio::test]
  async fn test_env_var_producer_mixed_existent_nonexistent() {
    let _lock = ENV_LOCK.lock().unwrap();

    unsafe {
      env::set_var("EXISTENT_VAR", "exists");
    }

    let producer = EnvVarProducer::with_vars(vec![
      "EXISTENT_VAR".to_string(),
      "NONEXISTENT_VAR".to_string(),
    ]);
    let mut producer = producer;
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.collect().await;

    assert_eq!(result.len(), 1);
    assert_eq!(
      result[0],
      ("EXISTENT_VAR".to_string(), "exists".to_string())
    );

    unsafe {
      env::remove_var("EXISTENT_VAR");
    }
  }

  #[tokio::test]
  async fn test_env_var_producer_all_env_vars() {
    let _lock = ENV_LOCK.lock().unwrap();

    // Set some test environment variables
    unsafe {
      env::set_var("TEST_ALL_1", "value1");
      env::set_var("TEST_ALL_2", "value2");
    }

    let mut producer = EnvVarProducer::new();
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.collect().await;

    // Should contain our test vars (and potentially others)
    assert!(
      result
        .iter()
        .any(|(k, v)| k == "TEST_ALL_1" && v == "value1")
    );
    assert!(
      result
        .iter()
        .any(|(k, v)| k == "TEST_ALL_2" && v == "value2")
    );

    unsafe {
      env::remove_var("TEST_ALL_1");
      env::remove_var("TEST_ALL_2");
    }
  }

  #[tokio::test]
  async fn test_env_var_producer_custom_error_handler() {
    let custom_handler = |_error: &StreamError<(String, String)>| ErrorAction::Skip;
    let producer =
      EnvVarProducer::new().with_error_strategy(ErrorStrategy::new_custom(custom_handler));

    let error = StreamError {
      source: Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "test error",
      )),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "TestEnvVarProducer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "TestEnvVarProducer".to_string(),
      },
      retries: 0,
    };

    // The custom error handler should return Skip
    let action = producer.handle_error(&error);
    assert_eq!(action, ErrorAction::Skip);
  }

  // Property-based tests using proptest
  proptest! {
      #[test]
      fn test_env_var_producer_properties(
          _var_names in prop::collection::vec(prop::string::string_regex("[A-Z_][A-Z0-9_]*").unwrap(), 0..10),
          _var_values in prop::collection::vec(prop::string::string_regex(".*").unwrap(), 0..10),
          custom_name in prop::option::of(prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]*").unwrap())
      ) {
          // Test that producer can be created with various configurations
          let mut producer = EnvVarProducer::new();

          if let Some(name) = custom_name {
              producer = producer.with_name(name.clone());
              assert_eq!(producer.config.name(), Some(name));
          }

          // Test error strategy variations
          let strategies = vec![
              ErrorStrategy::Stop,
              ErrorStrategy::Skip,
              ErrorStrategy::Retry(5),
          ];

          for strategy in strategies {
              let mut test_producer = producer.clone();
              test_producer = test_producer.with_error_strategy(strategy.clone());
              assert_eq!(test_producer.config.error_strategy(), strategy);
          }
      }

      #[test]
      fn test_env_var_producer_filter_properties(
          filter_vars in prop::collection::vec(prop::string::string_regex("[A-Z_][A-Z0-9_]*").unwrap(), 0..20)
      ) {
          let producer = EnvVarProducer::with_vars(filter_vars.clone());
          assert_eq!(producer.filter, Some(filter_vars));
      }

      #[test]
      fn test_env_var_producer_error_handling_properties(
          retry_count in 0..10usize,
          error_retries in 0..15usize
      ) {
          let producer = EnvVarProducer::new()
              .with_error_strategy(ErrorStrategy::Retry(retry_count));

          let error = StreamError {
              source: Box::new(std::io::Error::new(
                  std::io::ErrorKind::NotFound,
                  "test error",
              )),
              context: ErrorContext {
                  timestamp: chrono::Utc::now(),
                  item: None,
                  component_name: "test".to_string(),
                  component_type: "TestEnvVarProducer".to_string(),
              },
              component: ComponentInfo {
                  name: "test".to_string(),
                  type_name: "TestEnvVarProducer".to_string(),
              },
              retries: error_retries,
          };

          let action = producer.handle_error(&error);

          if error_retries < retry_count {
              assert_eq!(action, ErrorAction::Retry);
          } else {
              assert_eq!(action, ErrorAction::Stop);
          }
      }

      #[test]
      fn test_env_var_producer_config_properties(
          name in prop::option::of(prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_]*").unwrap()),
          strategy in prop::sample::select(vec![
              ErrorStrategy::Stop,
              ErrorStrategy::Skip,
              ErrorStrategy::Retry(3),
          ])
      ) {
          let mut producer = EnvVarProducer::new();

          if let Some(name_val) = &name {
              producer = producer.with_name(name_val.clone());
              assert_eq!(producer.config.name(), Some(name_val.clone()));
          }

          producer = producer.with_error_strategy(strategy.clone());
          assert_eq!(producer.config.error_strategy(), strategy);
      }
  }

  #[tokio::test]
  async fn test_env_var_producer_clone() {
    let producer = EnvVarProducer::new()
      .with_name("test_name".to_string())
      .with_error_strategy(ErrorStrategy::Skip);

    let cloned = producer.clone();
    assert_eq!(cloned.config.name(), producer.config.name());
    assert_eq!(
      cloned.config.error_strategy(),
      producer.config.error_strategy()
    );
  }

  #[tokio::test]
  async fn test_env_var_producer_debug() {
    let producer = EnvVarProducer::new().with_name("debug_test".to_string());
    let debug_str = format!("{:?}", producer);
    assert!(debug_str.contains("debug_test"));
  }

  #[tokio::test]
  async fn test_env_var_producer_edge_cases() {
    // Test with very long variable names and values
    let _lock = ENV_LOCK.lock().unwrap();

    let long_name = "A".repeat(1000);
    let long_value = "B".repeat(1000);

    unsafe {
      env::set_var(&long_name, &long_value);
    }

    let producer = EnvVarProducer::with_vars(vec![long_name.clone()]);
    let mut producer = producer;
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.collect().await;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], (long_name.clone(), long_value));

    unsafe {
      env::remove_var(&long_name);
    }
  }

  #[tokio::test]
  async fn test_env_var_producer_special_characters() {
    let _lock = ENV_LOCK.lock().unwrap();

    // Test with special characters in names and values
    let special_name = "SPECIAL_CHARS_!@#$%^&*()";
    let special_value = "value with spaces and special chars: !@#$%^&*()";

    unsafe {
      env::set_var(special_name, special_value);
    }

    let producer = EnvVarProducer::with_vars(vec![special_name.to_string()]);
    let mut producer = producer;
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.collect().await;

    assert_eq!(result.len(), 1);
    assert_eq!(
      result[0],
      (special_name.to_string(), special_value.to_string())
    );

    unsafe {
      env::remove_var(special_name);
    }
  }

  #[tokio::test]
  async fn test_env_var_producer_unicode() {
    let _lock = ENV_LOCK.lock().unwrap();

    // Test with Unicode characters
    let unicode_name = "UNICODE_测试_🚀";
    let unicode_value = "value with 测试 and 🚀 emoji";

    unsafe {
      env::set_var(unicode_name, unicode_value);
    }

    let producer = EnvVarProducer::with_vars(vec![unicode_name.to_string()]);
    let mut producer = producer;
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.collect().await;

    assert_eq!(result.len(), 1);
    assert_eq!(
      result[0],
      (unicode_name.to_string(), unicode_value.to_string())
    );

    unsafe {
      env::remove_var(unicode_name);
    }
  }
}
