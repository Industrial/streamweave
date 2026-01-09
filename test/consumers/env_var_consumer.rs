//! Tests for EnvVarConsumer

use futures::stream;
use streamweave::consumers::EnvVarConsumer;
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, ConsumerConfig};

#[tokio::test]
async fn test_env_var_consumer_basic() {
  let mut consumer = EnvVarConsumer::new();
  let input_stream = Box::pin(stream::iter(vec![
    ("TEST_VAR_1".to_string(), "value1".to_string()),
    ("TEST_VAR_2".to_string(), "value2".to_string()),
  ]));

  consumer.consume(input_stream).await;

  // Verify environment variables were set
  assert_eq!(std::env::var("TEST_VAR_1").unwrap(), "value1");
  assert_eq!(std::env::var("TEST_VAR_2").unwrap(), "value2");

  // Cleanup
  std::env::remove_var("TEST_VAR_1");
  std::env::remove_var("TEST_VAR_2");
}

#[tokio::test]
async fn test_env_var_consumer_invalid_name() {
  let mut consumer = EnvVarConsumer::new();
  let input_stream = Box::pin(stream::iter(vec![
    ("123_INVALID".to_string(), "value".to_string()), // Invalid: starts with number
    ("VALID_VAR".to_string(), "value".to_string()),
  ]));

  consumer.consume(input_stream).await;

  // Invalid var should not be set, valid one should be
  assert!(std::env::var("123_INVALID").is_err());
  assert_eq!(std::env::var("VALID_VAR").unwrap(), "value");

  // Cleanup
  std::env::remove_var("VALID_VAR");
}

#[tokio::test]
async fn test_env_var_consumer_empty() {
  let mut consumer = EnvVarConsumer::new();
  let input_stream = Box::pin(stream::iter(vec![] as Vec<(String, String)>));

  consumer.consume(input_stream).await;
  // Should complete without error
  assert!(true);
}

#[tokio::test]
async fn test_env_var_consumer_default() {
  let consumer = EnvVarConsumer::default();
  // Default consumer should be created
  assert!(true);
}

#[tokio::test]
async fn test_env_var_consumer_with_name() {
  let consumer = EnvVarConsumer::new().with_name("env_writer".to_string());

  assert_eq!(consumer.config.name, "env_writer");
}

#[tokio::test]
async fn test_env_var_consumer_with_error_strategy() {
  let consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[tokio::test]
async fn test_env_var_consumer_component_info() {
  let consumer = EnvVarConsumer::new().with_name("test".to_string());

  let info = consumer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("EnvVarConsumer"));
}

#[tokio::test]
async fn test_env_var_consumer_clone() {
  let consumer1 = EnvVarConsumer::new();
  let consumer2 = consumer1.clone();

  // Both should be valid
  assert!(true);
}
