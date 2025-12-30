#[cfg(feature = "kafka")]
use proptest::prelude::*;
#[cfg(feature = "kafka")]
use proptest::proptest;
use serde::Serialize;
use streamweave_kafka::{KafkaConsumer, KafkaProducerConfig};

#[derive(Debug, Clone, Serialize)]
struct TestEvent {
  id: u32,
  message: String,
}

#[cfg(feature = "kafka")]
proptest! {
  #[test]
  fn test_kafka_producer_config_default(_ in prop::num::u8::ANY) {
    let config = KafkaProducerConfig::default();
    prop_assert_eq!(config.bootstrap_servers, "localhost:9092");
    prop_assert_eq!(config.acks, "all");
    prop_assert_eq!(config.retries, 3);
  }

  #[test]
  fn test_kafka_producer_config_builder(
    bootstrap_servers in "[a-zA-Z0-9.-]+:[0-9]+",
    topic in "[a-zA-Z0-9-_]+",
    client_id in prop::string::string_regex("[a-zA-Z0-9-_]+").unwrap(),
    acks in prop::sample::select(vec!["0", "1", "all"]),
    retries in 0u32..10u32,
    compression in prop::sample::select(vec!["none", "gzip", "snappy", "lz4", "zstd"])
  ) {
    let config = KafkaProducerConfig::default()
      .with_bootstrap_servers(bootstrap_servers.clone())
      .with_topic(topic.clone())
      .with_client_id(client_id.clone())
      .with_acks(acks)
      .with_retries(retries)
      .with_compression_type(compression);

    prop_assert_eq!(config.bootstrap_servers, bootstrap_servers);
    prop_assert_eq!(config.topic, topic);
    prop_assert_eq!(config.client_id, Some(client_id));
    prop_assert_eq!(config.acks.as_str(), acks);
    prop_assert_eq!(config.retries, retries);
    prop_assert_eq!(config.compression_type.as_str(), compression);
  }

  #[test]
  fn test_kafka_consumer_new(
    topic in "[a-zA-Z0-9-_]+"
  ) {
    let kafka_config = KafkaProducerConfig::default().with_topic(topic.clone());
    let consumer = KafkaConsumer::<TestEvent>::new(kafka_config);
    prop_assert_eq!(consumer.kafka_config().topic.as_str(), topic.as_str());
  }
}

#[test]
fn test_kafka_producer_config_with_batch_size() {
  let config = KafkaProducerConfig::default().with_batch_size(32768);
  assert_eq!(config.batch_size, 32768);
}

#[test]
fn test_kafka_producer_config_with_linger_ms() {
  let config = KafkaProducerConfig::default().with_linger_ms(100);
  assert_eq!(config.linger_ms, 100);
}

#[test]
fn test_kafka_producer_config_with_max_request_size() {
  let config = KafkaProducerConfig::default().with_max_request_size(2097152);
  assert_eq!(config.max_request_size, 2097152);
}

#[test]
fn test_kafka_producer_config_with_custom_property() {
  let config = KafkaProducerConfig::default()
    .with_custom_property("key1", "value1")
    .with_custom_property("key2", "value2");
  assert_eq!(
    config.custom_properties.get("key1"),
    Some(&"value1".to_string())
  );
  assert_eq!(
    config.custom_properties.get("key2"),
    Some(&"value2".to_string())
  );
}

#[test]
fn test_kafka_consumer_builder() {
  let kafka_config = KafkaProducerConfig::default()
    .with_topic("test-topic")
    .with_bootstrap_servers("kafka:9092");
  let consumer = KafkaConsumer::<TestEvent>::new(kafka_config)
    .with_name("my_consumer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);
  assert_eq!(consumer.config().name, "my_consumer".to_string());
  assert!(matches!(
    consumer.config().error_strategy,
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_kafka_consumer_clone() {
  let kafka_config = KafkaProducerConfig::default().with_topic("test-topic");
  let consumer = KafkaConsumer::<TestEvent>::new(kafka_config).with_name("test".to_string());
  let cloned = consumer.clone();
  assert_eq!(cloned.config().name, consumer.config().name);
  assert_eq!(cloned.kafka_config().topic, consumer.kafka_config().topic);
}

#[test]
fn test_kafka_consumer_config_methods() {
  use streamweave::Consumer;
  use streamweave_error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let mut consumer =
    KafkaConsumer::<TestEvent>::new(KafkaProducerConfig::default().with_topic("test-topic"));
  let config = consumer.config().clone();
  assert_eq!(config.name, String::new());

  let config_mut = consumer.config_mut();
  config_mut.name = "test".to_string();
  assert_eq!(consumer.config().name, "test".to_string());

  let new_config = streamweave::ConsumerConfig::default();
  consumer.set_config(new_config.clone());
  assert_eq!(consumer.config().name, new_config.name);
}

#[test]
fn test_kafka_consumer_handle_error() {
  use streamweave::Consumer;
  use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

  let consumer =
    KafkaConsumer::<TestEvent>::new(KafkaProducerConfig::default().with_topic("test-topic"))
      .with_error_strategy(ErrorStrategy::Skip);
  let error = StreamError::new(
    Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test")),
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item: None,
      component_name: "test".to_string(),
      component_type: "test".to_string(),
    },
    ComponentInfo {
      name: "test".to_string(),
      type_name: "test".to_string(),
    },
  );
  assert_eq!(consumer.handle_error(&error), ErrorAction::Skip);

  let consumer_stop =
    KafkaConsumer::<TestEvent>::new(KafkaProducerConfig::default().with_topic("test-topic"))
      .with_error_strategy(ErrorStrategy::Stop);
  assert_eq!(consumer_stop.handle_error(&error), ErrorAction::Stop);

  let consumer_retry =
    KafkaConsumer::<TestEvent>::new(KafkaProducerConfig::default().with_topic("test-topic"))
      .with_error_strategy(ErrorStrategy::Retry(3));
  let mut error_retry = error.clone();
  error_retry.retries = 0;
  assert_eq!(
    consumer_retry.handle_error(&error_retry),
    ErrorAction::Retry
  );

  let mut error_retry_max = error.clone();
  error_retry_max.retries = 3;
  assert_eq!(
    consumer_retry.handle_error(&error_retry_max),
    ErrorAction::Stop
  );
}

#[test]
fn test_kafka_consumer_create_error_context() {
  use streamweave::Consumer;

  let consumer =
    KafkaConsumer::<TestEvent>::new(KafkaProducerConfig::default().with_topic("test-topic"))
      .with_name("test_consumer".to_string());
  let ctx = consumer.create_error_context(Some(TestEvent {
    id: 42,
    message: "test".to_string(),
  }));
  assert_eq!(ctx.component_name, "test_consumer");

  let ctx_no_item = consumer.create_error_context(None);
  assert_eq!(ctx_no_item.component_name, "test_consumer");
  assert_eq!(ctx_no_item.item, None);
}

#[test]
fn test_kafka_consumer_component_info() {
  use streamweave::Consumer;

  let consumer =
    KafkaConsumer::<TestEvent>::new(KafkaProducerConfig::default().with_topic("test-topic"))
      .with_name("my_consumer".to_string());
  let info = consumer.component_info();
  assert_eq!(info.name, "my_consumer");
  assert_eq!(
    info.type_name,
    std::any::type_name::<KafkaConsumer<TestEvent>>()
  );
}

#[test]
fn test_kafka_consumer_default_name() {
  use streamweave::Consumer;

  let consumer =
    KafkaConsumer::<TestEvent>::new(KafkaProducerConfig::default().with_topic("test-topic"));
  let info = consumer.component_info();
  assert_eq!(info.name, "kafka_consumer");
}
