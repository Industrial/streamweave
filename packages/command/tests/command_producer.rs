//! Tests for CommandProducer

use futures::StreamExt;
use streamweave::Producer;
use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use streamweave_command::CommandProducer;

#[tokio::test]
async fn test_command_producer_echo() {
  let mut producer = CommandProducer::new("echo", vec!["Hello, World!"]);
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;
  assert_eq!(result, vec!["Hello, World!"]);
}

#[tokio::test]
#[cfg(unix)] // This test is Unix-specific
async fn test_command_producer_multiple_lines() {
  let mut producer = CommandProducer::new("sh", vec!["-c", "echo 'line1\nline2\nline3'"]);
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;
  assert_eq!(result, vec!["line1", "line2", "line3"]);
}

#[tokio::test]
async fn test_command_producer_nonexistent() {
  let mut producer = CommandProducer::new("nonexistentcommand", Vec::<String>::new());
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;
  assert!(result.is_empty());
}

#[tokio::test]
async fn test_multiple_calls() {
  let mut producer = CommandProducer::new("echo", vec!["test"]);

  // First call
  let stream = producer.produce();
  let result1: Vec<String> = stream.collect().await;
  assert_eq!(result1, vec!["test"]);

  // Second call
  let stream = producer.produce();
  let result2: Vec<String> = stream.collect().await;
  assert_eq!(result2, vec!["test"]);
}

#[tokio::test]
#[cfg(unix)]
async fn test_command_error_handling() {
  let mut producer = CommandProducer::new("sh", vec!["-c", "echo test && false"]);
  let stream = producer.produce();
  let result: Vec<String> = stream.collect().await;
  assert_eq!(result, vec!["test"]);
}

#[tokio::test]
async fn test_error_handling_strategies() {
  let producer = CommandProducer::new("echo", vec!["test"])
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("test_producer".to_string());

  let config = producer.config();
  assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
  assert_eq!(config.name(), Some("test_producer".to_string()));

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "CommandProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "CommandProducer".to_string(),
    },
    retries: 0,
  };

  assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
}

#[tokio::test]
async fn test_with_name() {
  let producer =
    CommandProducer::new("echo", vec!["test"]).with_name("custom_producer".to_string());

  assert_eq!(
    producer.config().name(),
    Some("custom_producer".to_string())
  );
}

#[tokio::test]
async fn test_config_methods() {
  let mut producer = CommandProducer::new("echo", vec!["test"]);

  // Test get_config_impl
  let config = producer.get_config_impl();
  assert_eq!(config.error_strategy(), ErrorStrategy::Stop);

  // Test get_config_mut_impl
  let config_mut = producer.get_config_mut_impl();
  config_mut.error_strategy = ErrorStrategy::Skip;

  // Test set_config_impl
  let new_config = streamweave::ProducerConfig::default();
  producer.set_config_impl(new_config);

  assert_eq!(producer.config().error_strategy(), ErrorStrategy::Stop);
}

#[tokio::test]
async fn test_create_error_context_with_none() {
  let producer = CommandProducer::new("echo", vec!["test"]);
  let context = producer.create_error_context(None);

  assert_eq!(context.item, None);
  assert_eq!(context.item, None);
  assert_eq!(context.component_name, "command_producer");
  assert!(context.component_type.contains("CommandProducer"));
}

#[tokio::test]
async fn test_create_error_context_with_item() {
  let producer = CommandProducer::new("echo", vec!["test"]);
  let context = producer.create_error_context(Some("test_item".to_string()));

  assert_eq!(context.item, Some("test_item".to_string()));
  assert_eq!(context.component_name, "command_producer");
}

#[tokio::test]
async fn test_component_info_default() {
  let producer = CommandProducer::new("echo", vec!["test"]);
  let info = producer.component_info();

  assert_eq!(info.name, "command_producer");
  assert!(info.type_name.contains("CommandProducer"));
}

#[tokio::test]
async fn test_component_info_with_name() {
  let producer =
    CommandProducer::new("echo", vec!["test"]).with_name("custom_producer".to_string());
  let info = producer.component_info();

  assert_eq!(info.name, "custom_producer");
}

#[tokio::test]
async fn test_error_context_with_custom_name() {
  let producer =
    CommandProducer::new("echo", vec!["test"]).with_name("custom_producer".to_string());
  let context = producer.create_error_context(Some("test_item".to_string()));

  assert_eq!(context.component_name, "custom_producer");
}

#[tokio::test]
async fn test_handle_error_stop_strategy() {
  let producer =
    CommandProducer::new("echo", vec!["test"]).with_error_strategy(ErrorStrategy::Stop);

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "CommandProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "CommandProducer".to_string(),
    },
    retries: 0,
  };

  assert_eq!(producer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_handle_error_retry_strategy_under_limit() {
  let producer =
    CommandProducer::new("echo", vec!["test"]).with_error_strategy(ErrorStrategy::Retry(3));

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "CommandProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "CommandProducer".to_string(),
    },
    retries: 1, // Less than max (3)
  };

  assert_eq!(producer.handle_error(&error), ErrorAction::Retry);
}

#[tokio::test]
async fn test_handle_error_retry_strategy_at_limit() {
  let producer =
    CommandProducer::new("echo", vec!["test"]).with_error_strategy(ErrorStrategy::Retry(3));

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "CommandProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "CommandProducer".to_string(),
    },
    retries: 3, // At max (3)
  };

  assert_eq!(producer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_handle_error_retry_strategy_over_limit() {
  let producer =
    CommandProducer::new("echo", vec!["test"]).with_error_strategy(ErrorStrategy::Retry(3));

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "CommandProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "CommandProducer".to_string(),
    },
    retries: 4, // Over max (3)
  };

  assert_eq!(producer.handle_error(&error), ErrorAction::Stop);
}

#[tokio::test]
async fn test_handle_error_custom_strategy() {
  let custom_handler = |_error: &StreamError<String>| ErrorAction::Retry;
  let producer = CommandProducer::new("echo", vec!["test"])
    .with_error_strategy(ErrorStrategy::new_custom(custom_handler));

  let error = StreamError {
    source: Box::new(std::io::Error::other("test error")),
    context: ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "CommandProducer".to_string(),
    },
    component: ComponentInfo {
      name: "test".to_string(),
      type_name: "CommandProducer".to_string(),
    },
    retries: 0,
  };

  assert_eq!(producer.handle_error(&error), ErrorAction::Retry);
}

#[tokio::test]
async fn test_empty_output() {
  // Test command that produces no output
  #[cfg(unix)]
  {
    let mut producer = CommandProducer::new("true", Vec::<String>::new());
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert!(result.is_empty());
  }

  #[cfg(windows)]
  {
    let mut producer = CommandProducer::new("cmd", vec!["/C", "exit"]);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert!(result.is_empty());
  }
}

#[tokio::test]
async fn test_new_with_different_arg_types() {
  // Test that new() accepts different types that implement Into<String>
  let producer1 = CommandProducer::new("echo", vec!["test"]);
  let producer2 = CommandProducer::new("echo".to_string(), vec!["test".to_string()]);

  assert_eq!(producer1.command, producer2.command);
  assert_eq!(producer1.args, producer2.args);
}

#[tokio::test]
async fn test_output_trait_implementation() {
  // Test that CommandProducer implements Output trait correctly
  let producer = CommandProducer::new("echo", vec!["test"]);

  // Verify Output trait bounds
  fn assert_output_trait<P: streamweave::Output<Output = String>>(_producer: P)
  where
    <P as streamweave::Output>::Output: std::fmt::Debug + Clone + Send + Sync,
  {
  }
  assert_output_trait(producer);
}

#[tokio::test]
async fn test_producer_trait_implementation() {
  // Test that CommandProducer implements Producer trait correctly
  let mut producer = CommandProducer::new("echo", vec!["test"]);

  // Verify Producer trait bounds
  fn assert_producer_trait<P: streamweave::Producer<Output = String>>(_producer: &mut P)
  where
    <P as streamweave::Output>::Output: std::fmt::Debug + Clone + Send + Sync,
  {
  }
  assert_producer_trait(&mut producer);
}

#[tokio::test]
async fn test_output_ports() {
  let producer = CommandProducer::new("echo", vec!["test"]);

  // Verify OutputPorts is (String,)
  fn assert_output_ports<P: streamweave::Producer<OutputPorts = (String,)>>(_producer: &P)
  where
    <P as streamweave::Output>::Output: std::fmt::Debug + Clone + Send + Sync,
  {
  }
  assert_output_ports(&producer);
}
