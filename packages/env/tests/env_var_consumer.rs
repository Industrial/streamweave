//! Tests for EnvVarConsumer

use futures::Stream;
use futures::stream;
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::Consumer;
use streamweave_env::EnvVarConsumer;
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
async fn test_env_var_consumer_new() {
  let consumer = EnvVarConsumer::new();
  assert!(consumer.config.name.is_empty());
}

#[tokio::test]
async fn test_env_var_consumer_default() {
  let consumer = EnvVarConsumer::default();
  assert!(consumer.config.name.is_empty());
}

#[tokio::test]
async fn test_env_var_consumer_with_name() {
  let consumer = EnvVarConsumer::new().with_name("test_consumer".to_string());
  assert_eq!(consumer.config.name, "test_consumer");
}

#[tokio::test]
async fn test_env_var_consumer_with_error_strategy() {
  let consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Skip);
  match consumer.config.error_strategy {
    ErrorStrategy::Skip => {}
    _ => panic!("Expected Skip strategy"),
  }
}

#[tokio::test]
async fn test_env_var_consumer_consume_valid_vars() {
  // Use a unique test variable name
  let test_key = "STREAMWEAVE_TEST_CONSUME_VALID";
  let test_value = "test_value_123";

  // Ensure it doesn't exist before
  unsafe {
    std::env::remove_var(test_key);
  }

  let mut consumer = EnvVarConsumer::new();
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> = Box::pin(stream::iter(
    vec![(test_key.to_string(), test_value.to_string())],
  ));

  consumer.consume(input_stream).await;

  // Verify the variable was set
  assert_eq!(std::env::var(test_key).unwrap(), test_value);

  // Clean up
  unsafe {
    std::env::remove_var(test_key);
  }
}

#[tokio::test]
async fn test_env_var_consumer_consume_multiple_vars() {
  let test_key1 = "STREAMWEAVE_TEST_MULTI1";
  let test_value1 = "value1";
  let test_key2 = "STREAMWEAVE_TEST_MULTI2";
  let test_value2 = "value2";

  // Ensure they don't exist before
  unsafe {
    std::env::remove_var(test_key1);
    std::env::remove_var(test_key2);
  }

  let mut consumer = EnvVarConsumer::new();
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> =
    Box::pin(stream::iter(vec![
      (test_key1.to_string(), test_value1.to_string()),
      (test_key2.to_string(), test_value2.to_string()),
    ]));

  consumer.consume(input_stream).await;

  // Verify both variables were set
  assert_eq!(std::env::var(test_key1).unwrap(), test_value1);
  assert_eq!(std::env::var(test_key2).unwrap(), test_value2);

  // Clean up
  unsafe {
    std::env::remove_var(test_key1);
    std::env::remove_var(test_key2);
  }
}

#[tokio::test]
async fn test_env_var_consumer_consume_empty_stream() {
  let mut consumer = EnvVarConsumer::new();
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> =
    Box::pin(stream::empty());

  // Should complete without error
  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_env_var_consumer_consume_invalid_name_empty() {
  let mut consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Skip);
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> =
    Box::pin(stream::iter(vec![("".to_string(), "value".to_string())]));

  // Should skip invalid name and continue
  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_env_var_consumer_consume_invalid_name_starts_with_number() {
  let mut consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Skip);
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> = Box::pin(stream::iter(
    vec![("123INVALID".to_string(), "value".to_string())],
  ));

  // Should skip invalid name and continue
  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_env_var_consumer_consume_invalid_name_contains_space() {
  let mut consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Skip);
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> = Box::pin(stream::iter(
    vec![("INVALID NAME".to_string(), "value".to_string())],
  ));

  // Should skip invalid name and continue
  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_env_var_consumer_consume_invalid_name_contains_special_char() {
  let mut consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Skip);
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> = Box::pin(stream::iter(
    vec![("INVALID-NAME".to_string(), "value".to_string())],
  ));

  // Should skip invalid name and continue
  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_env_var_consumer_consume_invalid_name_stop_strategy() {
  let test_key = "STREAMWEAVE_TEST_INVALID_STOP";
  unsafe {
    std::env::remove_var(test_key);
  }

  let mut consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Stop);
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> =
    Box::pin(stream::iter(vec![
      ("VALID_NAME".to_string(), "value1".to_string()),
      ("".to_string(), "value2".to_string()), // Invalid - should stop
      ("VALID_NAME2".to_string(), "value3".to_string()), // Should not be processed
    ]));

  consumer.consume(input_stream).await;

  // First valid variable should be set
  assert_eq!(std::env::var("VALID_NAME").unwrap(), "value1");
  // Second valid variable should NOT be set (stopped early)
  assert!(std::env::var("VALID_NAME2").is_err());

  // Clean up
  unsafe {
    std::env::remove_var("VALID_NAME");
    std::env::remove_var(test_key);
  }
}

#[tokio::test]
async fn test_env_var_consumer_consume_invalid_name_retry_strategy() {
  let mut consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Retry(3));
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> =
    Box::pin(stream::iter(vec![("".to_string(), "value".to_string())]));

  // Retry strategy should skip validation errors (can't retry validation)
  consumer.consume(input_stream).await;
}

#[tokio::test]
async fn test_env_var_consumer_consume_valid_names() {
  let valid_names = vec![
    "VALID_NAME",
    "_VALID_NAME",
    "VALID_NAME_123",
    "A",
    "_",
    "VALID_NAME_WITH_UNDERSCORES",
  ];

  unsafe {
    for name in &valid_names {
      std::env::remove_var(name);
    }
  }

  let mut consumer = EnvVarConsumer::new();
  let items: Vec<(String, String)> = valid_names
    .iter()
    .map(|n| (n.to_string(), format!("value_{}", n)))
    .collect();
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> =
    Box::pin(stream::iter(items.clone()));

  consumer.consume(input_stream).await;

  // Verify all valid names were set
  for (name, value) in items {
    assert_eq!(std::env::var(&name).unwrap(), value);
    unsafe {
      std::env::remove_var(&name);
    }
  }
}

#[tokio::test]
async fn test_env_var_consumer_set_config_impl() {
  let mut consumer = EnvVarConsumer::new();
  let config = streamweave::ConsumerConfig {
    name: "test_name".to_string(),
    ..Default::default()
  };
  Consumer::set_config(&mut consumer, config);
  assert_eq!(consumer.config.name, "test_name");
}

#[tokio::test]
async fn test_env_var_consumer_get_config_impl() {
  let consumer = EnvVarConsumer::new().with_name("test".to_string());
  let config = Consumer::config(&consumer);
  assert_eq!(config.name, "test");
}

#[tokio::test]
async fn test_env_var_consumer_get_config_mut_impl() {
  let mut consumer = EnvVarConsumer::new();
  let config = Consumer::config_mut(&mut consumer);
  config.name = "mut_test".to_string();
  assert_eq!(consumer.config.name, "mut_test");
}

#[tokio::test]
async fn test_env_var_consumer_handle_error_stop() {
  let consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Stop);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(("key".to_string(), "value".to_string())),
      component_name: "test".to_string(),
      component_type: "EnvVarConsumer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "EnvVarConsumer".to_string(),
    },
    retries: 0,
  };

  let action = Consumer::handle_error(&consumer, &error);
  assert_eq!(action, ErrorAction::Stop);
}

#[tokio::test]
async fn test_env_var_consumer_handle_error_skip() {
  let consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Skip);
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(("key".to_string(), "value".to_string())),
      component_name: "test".to_string(),
      component_type: "EnvVarConsumer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "EnvVarConsumer".to_string(),
    },
    retries: 0,
  };

  let action = Consumer::handle_error(&consumer, &error);
  assert_eq!(action, ErrorAction::Skip);
}

#[tokio::test]
async fn test_env_var_consumer_handle_error_retry() {
  let consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Retry(3));
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(("key".to_string(), "value".to_string())),
      component_name: "test".to_string(),
      component_type: "EnvVarConsumer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "EnvVarConsumer".to_string(),
    },
    retries: 0,
  };

  let action = Consumer::handle_error(&consumer, &error);
  assert_eq!(action, ErrorAction::Retry);
}

#[tokio::test]
async fn test_env_var_consumer_handle_error_retry_exceeded() {
  let consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Retry(3));
  let error = StreamError {
    source: Box::new(TestError("test error".to_string())),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: Some(("key".to_string(), "value".to_string())),
      component_name: "test".to_string(),
      component_type: "EnvVarConsumer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "EnvVarConsumer".to_string(),
    },
    retries: 3, // Exceeded retry limit
  };

  let action = Consumer::handle_error(&consumer, &error);
  assert_eq!(action, ErrorAction::Stop);
}

#[tokio::test]
async fn test_env_var_consumer_create_error_context() {
  let consumer = EnvVarConsumer::new().with_name("test_consumer".to_string());
  let context =
    Consumer::create_error_context(&consumer, Some(("key".to_string(), "value".to_string())));
  assert_eq!(context.component_name, "test_consumer");
  assert_eq!(
    context.component_type,
    "streamweave_env::env_var_consumer::EnvVarConsumer"
  );
  assert_eq!(context.item, Some(("key".to_string(), "value".to_string())));
}

#[tokio::test]
async fn test_env_var_consumer_create_error_context_no_name() {
  let consumer = EnvVarConsumer::new();
  let context = Consumer::create_error_context(&consumer, None);
  assert_eq!(context.component_name, "");
  assert_eq!(
    context.component_type,
    "streamweave_env::env_var_consumer::EnvVarConsumer"
  );
  assert_eq!(context.item, None);
}

#[tokio::test]
async fn test_env_var_consumer_component_info() {
  let consumer = EnvVarConsumer::new().with_name("test_consumer".to_string());
  let info = Consumer::component_info(&consumer);
  assert_eq!(info.name, "test_consumer");
  assert_eq!(
    info.type_name,
    "streamweave_env::env_var_consumer::EnvVarConsumer"
  );
}

#[tokio::test]
async fn test_env_var_consumer_component_info_no_name() {
  let consumer = EnvVarConsumer::new();
  let info = Consumer::component_info(&consumer);
  assert_eq!(info.name, "");
  assert_eq!(
    info.type_name,
    "streamweave_env::env_var_consumer::EnvVarConsumer"
  );
}

#[tokio::test]
async fn test_env_var_consumer_input_trait() {
  let _consumer = EnvVarConsumer::new();
  // Verify Input trait is implemented
  let _: <EnvVarConsumer as streamweave::Input>::Input = ("key".to_string(), "value".to_string());
}

#[tokio::test]
async fn test_env_var_consumer_input_ports() {
  let mut consumer = EnvVarConsumer::new();
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> =
    Box::pin(stream::empty());
  consumer.consume(input_stream).await;
  // If this compiles, InputPorts is correctly implemented
}

// Test is_valid_env_var_name indirectly through consume
#[tokio::test]
async fn test_is_valid_env_var_name_valid_cases() {
  let valid_names = vec![
    "VALID",
    "_VALID",
    "VALID_123",
    "A",
    "_",
    "VALID_NAME",
    "VALID_NAME_123",
    "A1",
    "_1",
  ];

  for name in valid_names {
    unsafe {
      std::env::remove_var(name);
    }
    let mut consumer = EnvVarConsumer::new();
    let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> =
      Box::pin(stream::iter(vec![(name.to_string(), "value".to_string())]));
    consumer.consume(input_stream).await;
    // If we get here without error, the name was valid
    unsafe {
      std::env::remove_var(name);
    }
  }
}

#[tokio::test]
async fn test_is_valid_env_var_name_invalid_cases() {
  let invalid_names = vec![
    "",              // Empty
    "123INVALID",    // Starts with number
    "INVALID NAME",  // Contains space
    "INVALID-NAME",  // Contains hyphen
    "INVALID.NAME",  // Contains dot
    "INVALID@NAME",  // Contains @
    "INVALID#NAME",  // Contains #
    "INVALID$NAME",  // Contains $
    "INVALID%NAME",  // Contains %
    "INVALID&NAME",  // Contains &
    "INVALID*NAME",  // Contains *
    "INVALID(NAME",  // Contains (
    "INVALID)NAME",  // Contains )
    "INVALID+NAME",  // Contains +
    "INVALID=NAME",  // Contains =
    "INVALID[NAME",  // Contains [
    "INVALID]NAME",  // Contains ]
    "INVALID{NAME",  // Contains {
    "INVALID}NAME",  // Contains }
    "INVALID|NAME",  // Contains |
    "INVALID\\NAME", // Contains backslash
    "INVALID/NAME",  // Contains forward slash
    "INVALID:NAME",  // Contains colon
    "INVALID;NAME",  // Contains semicolon
    "INVALID<NAME",  // Contains <
    "INVALID>NAME",  // Contains >
    "INVALID?NAME",  // Contains ?
    "INVALID,NAME",  // Contains comma
    "INVALID'NAME",  // Contains single quote
    "INVALID\"NAME", // Contains double quote
    "INVALID`NAME",  // Contains backtick
    "INVALID~NAME",  // Contains tilde
    "INVALID!NAME",  // Contains !
    "INVALID^NAME",  // Contains ^
  ];

  for name in invalid_names {
    let mut consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Skip);
    let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> =
      Box::pin(stream::iter(vec![(name.to_string(), "value".to_string())]));
    // Should skip invalid names without error
    consumer.consume(input_stream).await;
  }
}

#[tokio::test]
async fn test_env_var_consumer_mixed_valid_invalid() {
  let test_key = "STREAMWEAVE_TEST_MIXED";
  unsafe {
    std::env::remove_var(test_key);
  }

  let mut consumer = EnvVarConsumer::new().with_error_strategy(ErrorStrategy::Skip);
  let input_stream: Pin<Box<dyn Stream<Item = (String, String)> + Send>> =
    Box::pin(stream::iter(vec![
      ("".to_string(), "value1".to_string()), // Invalid - skipped
      (test_key.to_string(), "valid_value".to_string()), // Valid - set
      ("123INVALID".to_string(), "value2".to_string()), // Invalid - skipped
      ("VALID_NAME".to_string(), "value3".to_string()), // Valid - set
    ]));

  consumer.consume(input_stream).await;

  // Verify valid variables were set
  assert_eq!(std::env::var(test_key).unwrap(), "valid_value");
  assert_eq!(std::env::var("VALID_NAME").unwrap(), "value3");

  // Clean up
  unsafe {
    std::env::remove_var(test_key);
    std::env::remove_var("VALID_NAME");
  }
}

// Property-based tests
proptest! {
  #[test]
  fn test_env_var_consumer_name_properties(
    name in "[a-zA-Z0-9_]+"
  ) {
    let consumer = EnvVarConsumer::new().with_name(name.clone());
    assert_eq!(consumer.config.name, name);
  }
}

// Note: Property-based async tests are complex, so we test valid env var names
// through the regular test above (test_env_var_consumer_consume_valid_names)
