use std::collections::HashMap;
use streamweave::error::ErrorStrategy;
use streamweave::producers::{RedisConsumerConfig, RedisMessage, RedisProducer};
use streamweave::{Producer, ProducerConfig};

#[test]
fn test_redis_consumer_config_default() {
  let config = RedisConsumerConfig::default();
  assert_eq!(config.connection_url, "redis://localhost:6379");
  assert_eq!(config.start_id, "0");
  assert_eq!(config.block_ms, 1000);
  assert!(!config.auto_ack);
  assert_eq!(config.stream, "");
  assert_eq!(config.group, None);
  assert_eq!(config.consumer, None);
  assert_eq!(config.count, None);
}

#[test]
fn test_redis_consumer_config_builder() {
  let config = RedisConsumerConfig::default()
    .with_connection_url("redis://redis:6379")
    .with_stream("test-stream")
    .with_group("test-group")
    .with_consumer("consumer-1")
    .with_start_id("$")
    .with_block_ms(5000)
    .with_count(100)
    .with_auto_ack(true);

  assert_eq!(config.connection_url, "redis://redis:6379");
  assert_eq!(config.stream, "test-stream");
  assert_eq!(config.group, Some("test-group".to_string()));
  assert_eq!(config.consumer, Some("consumer-1".to_string()));
  assert_eq!(config.start_id, "$");
  assert_eq!(config.block_ms, 5000);
  assert_eq!(config.count, Some(100));
  assert!(config.auto_ack);
}

#[test]
fn test_redis_consumer_config_builder_partial() {
  let config = RedisConsumerConfig::default()
    .with_connection_url("redis://localhost:6380")
    .with_stream("partial-stream")
    .with_block_ms(2000);

  assert_eq!(config.connection_url, "redis://localhost:6380");
  assert_eq!(config.stream, "partial-stream");
  assert_eq!(config.block_ms, 2000);
  assert_eq!(config.group, None);
  assert_eq!(config.consumer, None);
  assert!(!config.auto_ack);
}

#[test]
fn test_redis_consumer_config_builder_group_only() {
  let config = RedisConsumerConfig::default()
    .with_group("group-only")
    .with_consumer("consumer-only");

  assert_eq!(config.group, Some("group-only".to_string()));
  assert_eq!(config.consumer, Some("consumer-only".to_string()));
}

#[test]
fn test_redis_consumer_config_builder_count() {
  let config = RedisConsumerConfig::default().with_count(50);

  assert_eq!(config.count, Some(50));
}

#[test]
fn test_redis_consumer_config_clone() {
  let config = RedisConsumerConfig::default()
    .with_stream("test-stream")
    .with_group("test-group");
  let cloned = config.clone();
  assert_eq!(config.stream, cloned.stream);
  assert_eq!(config.group, cloned.group);
}

#[test]
fn test_redis_message_creation() {
  let mut fields = HashMap::new();
  fields.insert("field1".to_string(), "value1".to_string());
  fields.insert("field2".to_string(), "value2".to_string());

  let message = RedisMessage {
    stream: "test-stream".to_string(),
    id: "123-0".to_string(),
    fields: fields.clone(),
  };

  assert_eq!(message.stream, "test-stream");
  assert_eq!(message.id, "123-0");
  assert_eq!(message.fields, fields);
}

#[test]
fn test_redis_message_clone() {
  let mut fields = HashMap::new();
  fields.insert("key".to_string(), "value".to_string());
  let message = RedisMessage {
    stream: "stream".to_string(),
    id: "id".to_string(),
    fields,
  };
  let cloned = message.clone();
  assert_eq!(message.stream, cloned.stream);
  assert_eq!(message.id, cloned.id);
  assert_eq!(message.fields, cloned.fields);
}

#[test]
fn test_redis_message_serialize_deserialize() {
  use serde_json;
  let mut fields = HashMap::new();
  fields.insert("key1".to_string(), "value1".to_string());
  fields.insert("key2".to_string(), "value2".to_string());
  let message = RedisMessage {
    stream: "test-stream".to_string(),
    id: "123-0".to_string(),
    fields,
  };

  let serialized = serde_json::to_string(&message).unwrap();
  let deserialized: RedisMessage = serde_json::from_str(&serialized).unwrap();

  assert_eq!(message.stream, deserialized.stream);
  assert_eq!(message.id, deserialized.id);
  assert_eq!(message.fields, deserialized.fields);
}

#[test]
fn test_redis_producer_new() {
  let redis_config = RedisConsumerConfig::default().with_stream("test-stream");
  let producer = RedisProducer::new(redis_config.clone());
  assert_eq!(producer.redis_config().stream, "test-stream");
  assert_eq!(
    producer.redis_config().connection_url,
    redis_config.connection_url
  );
}

#[test]
fn test_redis_producer_with_error_strategy() {
  let redis_config = RedisConsumerConfig::default().with_stream("test-stream");
  let producer = RedisProducer::new(redis_config).with_error_strategy(ErrorStrategy::Retry(5));

  assert!(matches!(
    producer.config.error_strategy,
    ErrorStrategy::Retry(5)
  ));
}

#[test]
fn test_redis_producer_with_error_strategy_stop() {
  let redis_config = RedisConsumerConfig::default().with_stream("test-stream");
  let producer = RedisProducer::new(redis_config).with_error_strategy(ErrorStrategy::Stop);

  assert!(matches!(
    producer.config.error_strategy,
    ErrorStrategy::Stop
  ));
}

#[test]
fn test_redis_producer_with_error_strategy_skip() {
  let redis_config = RedisConsumerConfig::default().with_stream("test-stream");
  let producer = RedisProducer::new(redis_config).with_error_strategy(ErrorStrategy::Skip);

  assert!(matches!(
    producer.config.error_strategy,
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_redis_producer_with_error_strategy_custom() {
  let redis_config = RedisConsumerConfig::default().with_stream("test-stream");
  let custom_handler =
    |_error: &streamweave::error::StreamError<RedisMessage>| streamweave::error::ErrorAction::Skip;
  let producer =
    RedisProducer::new(redis_config).with_error_strategy(ErrorStrategy::new_custom(custom_handler));

  assert!(matches!(
    producer.config.error_strategy,
    ErrorStrategy::Custom(_)
  ));
}

#[test]
fn test_redis_producer_with_name() {
  let redis_config = RedisConsumerConfig::default().with_stream("test-stream");
  let producer = RedisProducer::new(redis_config).with_name("my-producer".to_string());

  assert_eq!(producer.config.name, Some("my-producer".to_string()));
}

#[test]
fn test_redis_producer_redis_config() {
  let redis_config = RedisConsumerConfig::default()
    .with_stream("test-stream")
    .with_group("test-group");
  let producer = RedisProducer::new(redis_config.clone());
  let retrieved_config = producer.redis_config();
  assert_eq!(retrieved_config.stream, redis_config.stream);
  assert_eq!(retrieved_config.group, redis_config.group);
}

#[test]
fn test_redis_producer_clone() {
  let redis_config = RedisConsumerConfig::default().with_stream("test-stream");
  let producer = RedisProducer::new(redis_config)
    .with_name("test-producer".to_string())
    .with_error_strategy(ErrorStrategy::Retry(3));
  let cloned = producer.clone();

  assert_eq!(producer.config.name, cloned.config.name);
  assert_eq!(producer.redis_config().stream, cloned.redis_config().stream);
}

#[test]
fn test_redis_producer_config_methods() {
  let redis_config = RedisConsumerConfig::default().with_stream("test-stream");
  let mut producer = RedisProducer::new(redis_config);

  // Test get_config_impl
  let config_ref = Producer::get_config_impl(&producer);
  assert_eq!(config_ref.name, producer.config.name);

  // Test get_config_mut_impl
  let config_mut = Producer::get_config_mut_impl(&mut producer);
  config_mut.name = Some("mutated-name".to_string());
  assert_eq!(producer.config.name, Some("mutated-name".to_string()));

  // Test set_config_impl
  let new_config = ProducerConfig::default();
  Producer::set_config_impl(&mut producer, new_config.clone());
  assert_eq!(producer.config.name, new_config.name);
}

#[test]
fn test_redis_producer_output_trait() {
  let redis_config = RedisConsumerConfig::default().with_stream("test-stream");
  let _producer = RedisProducer::new(redis_config);

  // Verify Output trait is implemented (compile-time check)
  use streamweave::Output;
  let _output_type = std::any::type_name::<<RedisProducer as Output>::Output>();
  // Just verify it compiles and returns a type name
  assert!(!_output_type.is_empty());
}

#[test]
fn test_redis_producer_output_ports() {
  let redis_config = RedisConsumerConfig::default().with_stream("test-stream");
  let _producer = RedisProducer::new(redis_config);

  // Verify Producer trait is implemented (compile-time check)
  use streamweave::Producer;
  let _ports_type = std::any::type_name::<<RedisProducer as Producer>::OutputPorts>();
  // This is a compile-time check, so if it compiles, the trait is implemented
  assert!(!_ports_type.is_empty());
}
