//! Tests for EnvVarProducer

use futures::StreamExt;
use proptest::prelude::*;
use streamweave::Producer;
use streamweave_env::EnvVarProducer;
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

// Test error type
#[derive(Debug)]
struct TestError(String);

impl std::fmt::Display for TestError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for TestError {}

#[tokio::test]
async fn test_env_var_producer_new() {
  let producer = EnvVarProducer::new();
  assert!(producer.filter.is_none());
  assert!(producer.config.name.is_none());
}

#[tokio::test]
async fn test_env_var_producer_default() {
  let producer = EnvVarProducer::default();
  assert!(producer.filter.is_none());
  assert!(producer.config.name.is_none());
}

#[tokio::test]
async fn test_env_var_producer_with_vars() {
  let vars = vec!["PATH".to_string(), "HOME".to_string()];
  let producer = EnvVarProducer::with_vars(vars.clone());
  assert_eq!(producer.filter, Some(vars));
}

#[tokio::test]
async fn test_env_var_producer_with_name() {
  let producer = EnvVarProducer::new().with_name("test_producer".to_string());
  assert_eq!(producer.config.name, Some("test_producer".to_string()));
}

#[tokio::test]
async fn test_env_var_producer_with_error_strategy() {
  let producer = EnvVarProducer::new().with_error_strategy(ErrorStrategy::Skip);
  match producer.config.error_strategy {
    ErrorStrategy::Skip => {}
    _ => panic!("Expected Skip strategy"),
  }
}

#[tokio::test]
async fn test_env_var_producer_produce_all_vars() {
  // Set some test environment variables
  unsafe {
    std::env::set_var("STREAMWEAVE_TEST_VAR1", "value1");
    std::env::set_var("STREAMWEAVE_TEST_VAR2", "value2");
  }

  let mut producer = EnvVarProducer::new();
  let stream = Producer::produce(&mut producer);
  let mut collected: Vec<(String, String)> = stream.collect().await;

  // Clean up
  unsafe {
    std::env::remove_var("STREAMWEAVE_TEST_VAR1");
    std::env::remove_var("STREAMWEAVE_TEST_VAR2");
  }

  // Verify we got at least our test variables
  collected.sort_by(|a, b| a.0.cmp(&b.0));
  let var1 = collected.iter().find(|(k, _)| k == "STREAMWEAVE_TEST_VAR1");
  let var2 = collected.iter().find(|(k, _)| k == "STREAMWEAVE_TEST_VAR2");

  assert!(var1.is_some(), "Should find STREAMWEAVE_TEST_VAR1");
  assert!(var2.is_some(), "Should find STREAMWEAVE_TEST_VAR2");
  assert_eq!(var1.unwrap().1, "value1");
  assert_eq!(var2.unwrap().1, "value2");
}

#[tokio::test]
async fn test_env_var_producer_produce_filtered_vars() {
  // Set some test environment variables
  unsafe {
    std::env::set_var("STREAMWEAVE_TEST_FILTER1", "filter_value1");
    std::env::set_var("STREAMWEAVE_TEST_FILTER2", "filter_value2");
    std::env::set_var("STREAMWEAVE_TEST_FILTER3", "filter_value3");
  }

  let mut producer = EnvVarProducer::with_vars(vec![
    "STREAMWEAVE_TEST_FILTER1".to_string(),
    "STREAMWEAVE_TEST_FILTER2".to_string(),
    "NONEXISTENT_VAR".to_string(), // This should be skipped
  ]);
  let stream = Producer::produce(&mut producer);
  let mut collected: Vec<(String, String)> = stream.collect().await;

  // Clean up
  unsafe {
    std::env::remove_var("STREAMWEAVE_TEST_FILTER1");
    std::env::remove_var("STREAMWEAVE_TEST_FILTER2");
    std::env::remove_var("STREAMWEAVE_TEST_FILTER3");
  }

  collected.sort_by(|a, b| a.0.cmp(&b.0));

  // Should only get the two existing variables
  assert_eq!(collected.len(), 2);
  assert_eq!(collected[0].0, "STREAMWEAVE_TEST_FILTER1");
  assert_eq!(collected[0].1, "filter_value1");
  assert_eq!(collected[1].0, "STREAMWEAVE_TEST_FILTER2");
  assert_eq!(collected[1].1, "filter_value2");
}

#[tokio::test]
async fn test_env_var_producer_produce_empty_filter() {
  let mut producer = EnvVarProducer::with_vars(vec![]);
  let stream = Producer::produce(&mut producer);
  let collected: Vec<(String, String)> = stream.collect().await;
  assert_eq!(collected.len(), 0);
}

#[tokio::test]
async fn test_env_var_producer_produce_nonexistent_vars() {
  let mut producer =
    EnvVarProducer::with_vars(vec!["STREAMWEAVE_NONEXISTENT_VAR_12345".to_string()]);
  let stream = Producer::produce(&mut producer);
  let collected: Vec<(String, String)> = stream.collect().await;
  // Non-existent vars should be filtered out
  assert_eq!(collected.len(), 0);
}

#[tokio::test]
async fn test_env_var_producer_set_config_impl() {
  let mut producer = EnvVarProducer::new();
  let config = streamweave::ProducerConfig {
    name: Some("test_name".to_string()),
    ..Default::default()
  };
  Producer::set_config(&mut producer, config);
  assert_eq!(producer.config.name, Some("test_name".to_string()));
}

#[tokio::test]
async fn test_env_var_producer_get_config_impl() {
  let producer = EnvVarProducer::new().with_name("test".to_string());
  let config = Producer::config(&producer);
  assert_eq!(config.name, Some("test".to_string()));
}

#[tokio::test]
async fn test_env_var_producer_get_config_mut_impl() {
  let mut producer = EnvVarProducer::new();
  let config = Producer::config_mut(&mut producer);
  config.name = Some("mut_test".to_string());
  assert_eq!(producer.config.name, Some("mut_test".to_string()));
}

#[tokio::test]
async fn test_env_var_producer_handle_error_stop() {
  let producer = EnvVarProducer::new().with_error_strategy(ErrorStrategy::Stop);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(("key".to_string(), "value".to_string())),
      component_name: "test".to_string(),
      component_type: "EnvVarProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "EnvVarProducer".to_string(),
    },
    retries: 0,
  };

  let action = Producer::handle_error(&producer, &error);
  assert_eq!(action, ErrorAction::Stop);
}

#[tokio::test]
async fn test_env_var_producer_handle_error_skip() {
  let producer = EnvVarProducer::new().with_error_strategy(ErrorStrategy::Skip);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(("key".to_string(), "value".to_string())),
      component_name: "test".to_string(),
      component_type: "EnvVarProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "EnvVarProducer".to_string(),
    },
    retries: 0,
  };

  let action = Producer::handle_error(&producer, &error);
  assert_eq!(action, ErrorAction::Skip);
}

#[tokio::test]
async fn test_env_var_producer_handle_error_retry() {
  let producer = EnvVarProducer::new().with_error_strategy(ErrorStrategy::Retry(3));
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(("key".to_string(), "value".to_string())),
      component_name: "test".to_string(),
      component_type: "EnvVarProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "EnvVarProducer".to_string(),
    },
    retries: 0,
  };

  let action = Producer::handle_error(&producer, &error);
  assert_eq!(action, ErrorAction::Retry);
}

#[tokio::test]
async fn test_env_var_producer_handle_error_retry_exceeded() {
  let producer = EnvVarProducer::new().with_error_strategy(ErrorStrategy::Retry(3));
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(("key".to_string(), "value".to_string())),
      component_name: "test".to_string(),
      component_type: "EnvVarProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "EnvVarProducer".to_string(),
    },
    retries: 3, // Exceeded retry limit
  };

  let action = Producer::handle_error(&producer, &error);
  assert_eq!(action, ErrorAction::Stop);
}

#[tokio::test]
async fn test_env_var_producer_handle_error_custom() {
  let custom_handler = |_error: &StreamError<(String, String)>| ErrorAction::Skip;
  let producer =
    EnvVarProducer::new().with_error_strategy(ErrorStrategy::new_custom(custom_handler));
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(("key".to_string(), "value".to_string())),
      component_name: "test".to_string(),
      component_type: "EnvVarProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "EnvVarProducer".to_string(),
    },
    retries: 0,
  };

  let action = Producer::handle_error(&producer, &error);
  assert_eq!(action, ErrorAction::Skip);
}

#[tokio::test]
async fn test_env_var_producer_create_error_context() {
  let producer = EnvVarProducer::new().with_name("test_producer".to_string());
  let context =
    Producer::create_error_context(&producer, Some(("key".to_string(), "value".to_string())));
  assert_eq!(context.component_name, "test_producer");
  assert_eq!(
    context.component_type,
    "streamweave_env::env_var_producer::EnvVarProducer"
  );
  assert_eq!(context.item, Some(("key".to_string(), "value".to_string())));
}

#[tokio::test]
async fn test_env_var_producer_create_error_context_no_name() {
  let producer = EnvVarProducer::new();
  let context = Producer::create_error_context(&producer, None);
  assert_eq!(context.component_name, "env_var_producer");
  assert_eq!(
    context.component_type,
    "streamweave_env::env_var_producer::EnvVarProducer"
  );
  assert_eq!(context.item, None);
}

#[tokio::test]
async fn test_env_var_producer_component_info() {
  let producer = EnvVarProducer::new().with_name("test_producer".to_string());
  let info = Producer::component_info(&producer);
  assert_eq!(info.name, "test_producer");
  assert_eq!(
    info.type_name,
    "streamweave_env::env_var_producer::EnvVarProducer"
  );
}

#[tokio::test]
async fn test_env_var_producer_component_info_no_name() {
  let producer = EnvVarProducer::new();
  let info = Producer::component_info(&producer);
  assert_eq!(info.name, "env_var_producer");
  assert_eq!(
    info.type_name,
    "streamweave_env::env_var_producer::EnvVarProducer"
  );
}

#[tokio::test]
async fn test_env_var_producer_multiple_produce_calls() {
  unsafe {
    std::env::set_var("STREAMWEAVE_TEST_MULTI", "multi_value");
  }

  let mut producer = EnvVarProducer::with_vars(vec!["STREAMWEAVE_TEST_MULTI".to_string()]);

  // First call
  let stream1 = Producer::produce(&mut producer);
  let collected1: Vec<(String, String)> = stream1.collect().await;
  assert_eq!(collected1.len(), 1);
  assert_eq!(collected1[0].0, "STREAMWEAVE_TEST_MULTI");
  assert_eq!(collected1[0].1, "multi_value");

  // Second call
  let stream2 = Producer::produce(&mut producer);
  let collected2: Vec<(String, String)> = stream2.collect().await;
  assert_eq!(collected2.len(), 1);
  assert_eq!(collected2[0].0, "STREAMWEAVE_TEST_MULTI");
  assert_eq!(collected2[0].1, "multi_value");

  unsafe {
    std::env::remove_var("STREAMWEAVE_TEST_MULTI");
  }
}

#[tokio::test]
async fn test_env_var_producer_output_trait() {
  let _producer = EnvVarProducer::new();
  // Verify Output trait is implemented
  let _: <EnvVarProducer as streamweave::Output>::Output = ("key".to_string(), "value".to_string());
}

#[tokio::test]
async fn test_env_var_producer_output_ports() {
  let mut producer = EnvVarProducer::new();
  // Verify OutputPorts is correctly set
  let _stream = Producer::produce(&mut producer);
  // If this compiles, OutputPorts is correctly implemented
}

// Property-based tests
proptest! {
  #[test]
  fn test_env_var_producer_with_vars_properties(
    var_names in prop::collection::vec("[A-Z_][A-Z0-9_]*", 0..10)
  ) {
    let vars: Vec<String> = var_names;
    let producer = EnvVarProducer::with_vars(vars.clone());
    assert_eq!(producer.filter, Some(vars));
  }

  #[test]
  fn test_env_var_producer_name_properties(
    name in "[a-zA-Z0-9_]+"
  ) {
    let producer = EnvVarProducer::new().with_name(name.clone());
    assert_eq!(producer.config.name, Some(name));
  }
}
