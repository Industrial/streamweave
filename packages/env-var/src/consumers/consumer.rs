use super::env_var_consumer::EnvVarConsumer;
use async_trait::async_trait;
use futures::StreamExt;
use streamweave::{Consumer, ConsumerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use tracing::{error, warn};

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

impl EnvVarConsumer {
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

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use proptest::prelude::*;
  use streamweave_error::ErrorStrategy;

  #[tokio::test]
  async fn test_env_var_consumer_basic() {
    let mut consumer = EnvVarConsumer::new();

    let test_vars = vec![
      ("TEST_VAR_1".to_string(), "value1".to_string()),
      ("TEST_VAR_2".to_string(), "value2".to_string()),
    ];

    let input_stream = stream::iter(test_vars.clone());
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;

    // Verify environment variables were set
    assert_eq!(std::env::var("TEST_VAR_1").unwrap(), "value1");
    assert_eq!(std::env::var("TEST_VAR_2").unwrap(), "value2");

    // Cleanup
    unsafe {
      std::env::remove_var("TEST_VAR_1");
      std::env::remove_var("TEST_VAR_2");
    }
  }

  #[tokio::test]
  async fn test_env_var_consumer_invalid_name() {
    let mut consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Skip);

    let test_vars = vec![
      ("INVALID-VAR".to_string(), "value1".to_string()), // Invalid: contains hyphen
      ("VALID_VAR".to_string(), "value2".to_string()),
    ];

    let input_stream = stream::iter(test_vars);
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;

    // Invalid var should be skipped, valid var should be set
    assert!(std::env::var("INVALID-VAR").is_err());
    assert_eq!(std::env::var("VALID_VAR").unwrap(), "value2");

    // Cleanup
    unsafe {
      std::env::remove_var("VALID_VAR");
    }
  }

  #[tokio::test]
  async fn test_env_var_consumer_stop_on_error() {
    let mut consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Stop);

    let test_vars = vec![
      ("INVALID VAR".to_string(), "value1".to_string()), // Invalid: contains space
      ("SHOULD_NOT_BE_SET".to_string(), "value2".to_string()),
    ];

    let input_stream = stream::iter(test_vars);
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;

    // Both should not be set because we stop on first error
    assert!(std::env::var("INVALID VAR").is_err());
    assert!(std::env::var("SHOULD_NOT_BE_SET").is_err());
  }

  #[tokio::test]
  async fn test_env_var_consumer_empty_stream() {
    let mut consumer = EnvVarConsumer::new();

    let input_stream = stream::iter(Vec::<(String, String)>::new());
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;

    // Should complete without errors
  }

  proptest! {
    #[test]
    fn test_error_handling_strategies(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = EnvVarConsumer::new()
        .with_error_strategy(ErrorStrategy::<(String, String)>::Skip)
        .with_name(name.clone());

      prop_assert!(matches!(consumer.config().error_strategy, ErrorStrategy::<(String, String)>::Skip));
      prop_assert_eq!(consumer.config().name.as_str(), name.as_str());
    }

    #[test]
    fn test_component_info(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = EnvVarConsumer::new().with_name(name.clone());
      let info = consumer.component_info();
      prop_assert_eq!(info.name, name);
      prop_assert_eq!(
        info.type_name,
        std::any::type_name::<EnvVarConsumer>()
      );
    }

    #[test]
    fn test_error_context_creation(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      key in prop::string::string_regex("[A-Z_][A-Z0-9_]*").unwrap(),
      value in prop::string::string_regex(".*").unwrap()
    ) {
      let consumer = EnvVarConsumer::new().with_name(name.clone());
      let item = (key.clone(), value.clone());
      let context = consumer.create_error_context(Some(item.clone()));
      prop_assert_eq!(context.component_name, name);
      prop_assert_eq!(
        context.component_type,
        std::any::type_name::<EnvVarConsumer>()
      );
      prop_assert_eq!(context.item, Some(item));
    }
  }

  #[test]
  fn test_is_valid_env_var_name() {
    // Valid names
    assert!(EnvVarConsumer::is_valid_env_var_name("VAR"));
    assert!(EnvVarConsumer::is_valid_env_var_name("_VAR"));
    assert!(EnvVarConsumer::is_valid_env_var_name("VAR_123"));
    assert!(EnvVarConsumer::is_valid_env_var_name("MY_VAR_NAME"));

    // Invalid names
    assert!(!EnvVarConsumer::is_valid_env_var_name(""));
    assert!(!EnvVarConsumer::is_valid_env_var_name("123VAR")); // Starts with digit
    assert!(!EnvVarConsumer::is_valid_env_var_name("VAR-NAME")); // Contains hyphen
    assert!(!EnvVarConsumer::is_valid_env_var_name("VAR NAME")); // Contains space
    assert!(!EnvVarConsumer::is_valid_env_var_name("VAR.NAME")); // Contains dot
  }
}
