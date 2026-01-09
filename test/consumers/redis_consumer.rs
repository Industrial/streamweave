use proptest::prelude::*;
use proptest::proptest;
use serde::Serialize;
use streamweave::consumers::{RedisConsumer, RedisProducerConfig};
use streamweave::error::ErrorStrategy;
use streamweave::{Consumer, ConsumerConfig};

#[derive(Debug, Clone, Serialize)]
struct TestEvent {
  id: u32,
  message: String,
}

#[test]
fn test_redis_producer_config_default() {
  let config = RedisProducerConfig::default();
  assert_eq!(config.connection_url, "redis://localhost:6379");
  assert_eq!(config.maxlen, None);
  assert!(!config.approximate_maxlen);
  assert_eq!(config.stream, "");
}

#[test]
fn test_redis_producer_config_builder_all() {
  let config = RedisProducerConfig::default()
    .with_connection_url("redis://redis:6379")
    .with_stream("test-stream")
    .with_maxlen(10000)
    .with_approximate_maxlen(true);

  assert_eq!(config.connection_url, "redis://redis:6379");
  assert_eq!(config.stream, "test-stream");
  assert_eq!(config.maxlen, Some(10000));
  assert!(config.approximate_maxlen);
}

#[test]
fn test_redis_producer_config_builder_partial() {
  let config = RedisProducerConfig::default()
    .with_connection_url("redis://localhost:6380")
    .with_stream("partial-stream");

  assert_eq!(config.connection_url, "redis://localhost:6380");
  assert_eq!(config.stream, "partial-stream");
  assert_eq!(config.maxlen, None);
  assert!(!config.approximate_maxlen);
}

#[test]
fn test_redis_producer_config_with_maxlen() {
  let config = RedisProducerConfig::default().with_maxlen(5000);

  assert_eq!(config.maxlen, Some(5000));
}

#[test]
fn test_redis_producer_config_with_approximate_maxlen() {
  let config = RedisProducerConfig::default().with_approximate_maxlen(true);

  assert!(config.approximate_maxlen);
}

#[test]
fn test_redis_producer_config_clone() {
  let config = RedisProducerConfig::default()
    .with_stream("test-stream")
    .with_maxlen(1000);
  let cloned = config.clone();
  assert_eq!(config.stream, cloned.stream);
  assert_eq!(config.maxlen, cloned.maxlen);
}

#[test]
fn test_redis_consumer_new() {
  let redis_config = RedisProducerConfig::default().with_stream("test-stream");
  let consumer = RedisConsumer::<TestEvent>::new(redis_config.clone());
  assert_eq!(consumer.redis_config().stream, redis_config.stream);
}

#[test]
fn test_redis_consumer_with_error_strategy() {
  let redis_config = RedisProducerConfig::default().with_stream("test-stream");
  let consumer =
    RedisConsumer::<TestEvent>::new(redis_config).with_error_strategy(ErrorStrategy::Retry(5));

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Retry(5)
  ));
}

#[test]
fn test_redis_consumer_with_error_strategy_stop() {
  let redis_config = RedisProducerConfig::default().with_stream("test-stream");
  let consumer =
    RedisConsumer::<TestEvent>::new(redis_config).with_error_strategy(ErrorStrategy::Stop);

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Stop
  ));
}

#[test]
fn test_redis_consumer_with_error_strategy_skip() {
  let redis_config = RedisProducerConfig::default().with_stream("test-stream");
  let consumer =
    RedisConsumer::<TestEvent>::new(redis_config).with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_redis_consumer_with_error_strategy_custom() {
  let redis_config = RedisProducerConfig::default().with_stream("test-stream");
  let custom_handler =
    |_error: &streamweave::error::StreamError<TestEvent>| streamweave::error::ErrorAction::Skip;
  let consumer = RedisConsumer::<TestEvent>::new(redis_config)
    .with_error_strategy(ErrorStrategy::new_custom(custom_handler));

  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Custom(_)
  ));
}

#[test]
fn test_redis_consumer_with_name() {
  let redis_config = RedisProducerConfig::default().with_stream("test-stream");
  let consumer = RedisConsumer::<TestEvent>::new(redis_config).with_name("my-consumer".to_string());

  assert_eq!(consumer.config.name, "my-consumer".to_string());
}

#[test]
fn test_redis_consumer_redis_config() {
  let redis_config = RedisProducerConfig::default()
    .with_stream("test-stream")
    .with_maxlen(5000);
  let consumer = RedisConsumer::<TestEvent>::new(redis_config.clone());
  let retrieved_config = consumer.redis_config();
  assert_eq!(retrieved_config.stream, redis_config.stream);
  assert_eq!(retrieved_config.maxlen, redis_config.maxlen);
}

#[test]
fn test_redis_consumer_clone() {
  let redis_config = RedisProducerConfig::default().with_stream("test-stream");
  let consumer = RedisConsumer::<TestEvent>::new(redis_config)
    .with_name("test-consumer".to_string())
    .with_error_strategy(ErrorStrategy::Retry(3));
  let cloned = consumer.clone();

  assert_eq!(consumer.config.name, cloned.config.name);
  assert_eq!(consumer.redis_config().stream, cloned.redis_config().stream);
}

#[test]
fn test_redis_consumer_config_methods() {
  let redis_config = RedisProducerConfig::default().with_stream("test-stream");
  let mut consumer = RedisConsumer::<TestEvent>::new(redis_config);

  // Test get_config_impl
  let config_ref = Consumer::get_config_impl(&consumer);
  assert_eq!(config_ref.name, consumer.config.name);

  // Test get_config_mut_impl
  let config_mut = Consumer::get_config_mut_impl(&mut consumer);
  config_mut.name = "mutated-name".to_string();
  assert_eq!(consumer.config.name, "mutated-name".to_string());

  // Test set_config_impl
  let new_config = ConsumerConfig::default();
  Consumer::set_config_impl(&mut consumer, new_config.clone());
  assert_eq!(consumer.config.name, new_config.name);
}

#[test]
fn test_redis_consumer_input_trait() {
  let redis_config = RedisProducerConfig::default().with_stream("test-stream");
  let _consumer = RedisConsumer::<TestEvent>::new(redis_config);

  // Verify Input trait is implemented (compile-time check)
  use streamweave::Input;
  let _input_type = std::any::type_name::<<RedisConsumer<TestEvent> as Input>::Input>();
  // Just verify it compiles and returns a type name
  assert!(!_input_type.is_empty());
}

#[test]
fn test_redis_consumer_input_ports() {
  let redis_config = RedisProducerConfig::default().with_stream("test-stream");
  let _consumer = RedisConsumer::<TestEvent>::new(redis_config);

  // Verify Consumer trait is implemented (compile-time check)
  use streamweave::Consumer;
  let _ports_type = std::any::type_name::<<RedisConsumer<TestEvent> as Consumer>::InputPorts>();
  // This is a compile-time check, so if it compiles, the trait is implemented
  assert!(!_ports_type.is_empty());
}

proptest! {
  #[test]
  fn test_redis_producer_config_default_prop(_ in prop::num::u8::ANY) {
    let config = RedisProducerConfig::default();
    prop_assert_eq!(config.connection_url, "redis://localhost:6379");
    prop_assert_eq!(config.maxlen, None);
    prop_assert!(!config.approximate_maxlen);
  }

  #[test]
  fn test_redis_producer_config_builder_prop(
    connection_url in "redis://[a-zA-Z0-9.-]+:[0-9]+",
    stream in "[a-zA-Z0-9-_]+",
    maxlen in prop::option::of(1000usize..100000usize),
    approximate_maxlen in prop::bool::ANY
  ) {
    let config = RedisProducerConfig::default()
      .with_connection_url(connection_url.clone())
      .with_stream(stream.clone())
      .with_approximate_maxlen(approximate_maxlen);

    let config = if let Some(max) = maxlen {
      config.with_maxlen(max)
    } else {
      config
    };

    prop_assert_eq!(config.connection_url, connection_url);
    prop_assert_eq!(config.stream, stream);
    prop_assert_eq!(config.maxlen, maxlen);
    prop_assert_eq!(config.approximate_maxlen, approximate_maxlen);
  }

  #[test]
  fn test_redis_consumer_new_prop(
    stream in "[a-zA-Z0-9-_]+"
  ) {
    let redis_config = RedisProducerConfig::default().with_stream(stream.clone());
    let consumer = RedisConsumer::<TestEvent>::new(redis_config);
    prop_assert_eq!(consumer.redis_config().stream.as_str(), stream.as_str());
  }
}
