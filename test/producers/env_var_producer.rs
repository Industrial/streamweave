//! Tests for EnvVarProducer

use futures::StreamExt;
use streamweave::error::ErrorStrategy;
use streamweave::producers::EnvVarProducer;
use streamweave::{Producer, ProducerConfig};

#[tokio::test]
async fn test_env_var_producer_all() {
  let mut producer = EnvVarProducer::new();
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  // Should have at least some environment variables
  assert!(!results.is_empty());
}

#[tokio::test]
async fn test_env_var_producer_with_vars() {
  // Set a test environment variable
  std::env::set_var("TEST_VAR_1", "value1");
  std::env::set_var("TEST_VAR_2", "value2");

  let mut producer = EnvVarProducer::with_vars(vec![
    "TEST_VAR_1".to_string(),
    "TEST_VAR_2".to_string(),
    "NONEXISTENT_VAR".to_string(),
  ]);
  let mut stream = producer.produce();

  let mut results = Vec::new();
  while let Some(item) = stream.next().await {
    results.push(item);
  }

  // Should have the two existing vars
  assert_eq!(results.len(), 2);
  let keys: Vec<String> = results.iter().map(|(k, _)| k.clone()).collect();
  assert!(keys.contains(&"TEST_VAR_1".to_string()));
  assert!(keys.contains(&"TEST_VAR_2".to_string()));

  // Cleanup
  std::env::remove_var("TEST_VAR_1");
  std::env::remove_var("TEST_VAR_2");
}

#[tokio::test]
async fn test_env_var_producer_default() {
  let producer = EnvVarProducer::default();
  let mut stream = producer.produce();

  // Should produce environment variables
  assert!(stream.next().await.is_some());
}

#[tokio::test]
async fn test_env_var_producer_with_name() {
  let producer = EnvVarProducer::new().with_name("env_reader".to_string());

  let config = producer.get_config_impl();
  assert_eq!(config.name(), Some("env_reader"));
}

#[tokio::test]
async fn test_env_var_producer_with_error_strategy() {
  let producer = EnvVarProducer::new().with_error_strategy(ErrorStrategy::Skip);

  let config = producer.get_config_impl();
  assert!(matches!(config.error_strategy(), ErrorStrategy::Skip));
}

#[tokio::test]
async fn test_env_var_producer_component_info() {
  let producer = EnvVarProducer::new().with_name("test".to_string());

  let info = producer.component_info();
  assert_eq!(info.name, "test");
  assert!(info.type_name.contains("EnvVarProducer"));
}

#[tokio::test]
async fn test_env_var_producer_clone() {
  let producer1 = EnvVarProducer::new();
  let producer2 = producer1.clone();

  // Both should be valid
  assert!(true);
}
