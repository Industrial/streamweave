//! Tests for error handling and edge cases

use serde::Serialize;
use streamweave::error::ErrorStrategy;
use streamweave_redis::{RedisConsumer, RedisProducerConfig};
use streamweave_redis::{RedisConsumerConfig, RedisProducer};

#[derive(Debug, Clone, Serialize)]
struct TestData {
  value: i32,
}

#[test]
fn test_redis_consumer_config_empty_stream() {
  let config = RedisConsumerConfig::default();
  assert_eq!(config.stream, "");
}

#[test]
fn test_redis_consumer_config_zero_block_ms() {
  let config = RedisConsumerConfig::default().with_block_ms(0);
  assert_eq!(config.block_ms, 0);
}

#[test]
fn test_redis_consumer_config_large_block_ms() {
  let config = RedisConsumerConfig::default().with_block_ms(u64::MAX);
  assert_eq!(config.block_ms, u64::MAX);
}

#[test]
fn test_redis_consumer_config_count_none() {
  let config = RedisConsumerConfig::default();
  assert_eq!(config.count, None);
}

#[test]
fn test_redis_consumer_config_count_zero() {
  let config = RedisConsumerConfig::default().with_count(0);
  assert_eq!(config.count, Some(0));
}

#[test]
fn test_redis_consumer_config_count_large() {
  let config = RedisConsumerConfig::default().with_count(usize::MAX);
  assert_eq!(config.count, Some(usize::MAX));
}

#[test]
fn test_redis_consumer_config_group_without_consumer() {
  let config = RedisConsumerConfig::default()
    .with_group("group")
    .with_consumer(None);
  assert_eq!(config.group, Some("group".to_string()));
  assert_eq!(config.consumer, None);
}

#[test]
fn test_redis_consumer_config_consumer_without_group() {
  let config = RedisConsumerConfig::default().with_consumer("consumer");
  assert_eq!(config.group, None);
  assert_eq!(config.consumer, Some("consumer".to_string()));
}

#[test]
fn test_redis_producer_config_empty_stream() {
  let config = RedisProducerConfig::default();
  assert_eq!(config.stream, "");
}

#[test]
fn test_redis_producer_config_maxlen_zero() {
  let config = RedisProducerConfig::default().with_maxlen(0);
  assert_eq!(config.maxlen, Some(0));
}

#[test]
fn test_redis_producer_config_maxlen_large() {
  let config = RedisProducerConfig::default().with_maxlen(usize::MAX);
  assert_eq!(config.maxlen, Some(usize::MAX));
}

#[test]
fn test_redis_producer_config_approximate_maxlen_false() {
  let config = RedisProducerConfig::default().with_approximate_maxlen(false);
  assert!(!config.approximate_maxlen);
}

#[test]
fn test_redis_producer_config_approximate_maxlen_true() {
  let config = RedisProducerConfig::default().with_approximate_maxlen(true);
  assert!(config.approximate_maxlen);
}

#[test]
fn test_redis_producer_config_with_maxlen_then_approximate() {
  let config = RedisProducerConfig::default()
    .with_maxlen(1000)
    .with_approximate_maxlen(true);
  assert_eq!(config.maxlen, Some(1000));
  assert!(config.approximate_maxlen);
}

#[test]
fn test_redis_producer_error_strategy_retry_zero() {
  let redis_config = RedisConsumerConfig::default().with_stream("test");
  let producer = RedisProducer::new(redis_config).with_error_strategy(ErrorStrategy::Retry(0));
  assert!(matches!(
    producer.config.error_strategy,
    ErrorStrategy::Retry(0)
  ));
}

#[test]
fn test_redis_producer_error_strategy_retry_large() {
  let redis_config = RedisConsumerConfig::default().with_stream("test");
  let producer =
    RedisProducer::new(redis_config).with_error_strategy(ErrorStrategy::Retry(usize::MAX));
  assert!(matches!(
    producer.config.error_strategy,
    ErrorStrategy::Retry(usize::MAX)
  ));
}

#[test]
fn test_redis_consumer_error_strategy_retry_zero() {
  let redis_config = RedisProducerConfig::default().with_stream("test");
  let consumer =
    RedisConsumer::<TestData>::new(redis_config).with_error_strategy(ErrorStrategy::Retry(0));
  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Retry(0)
  ));
}

#[test]
fn test_redis_consumer_error_strategy_retry_large() {
  let redis_config = RedisProducerConfig::default().with_stream("test");
  let consumer = RedisConsumer::<TestData>::new(redis_config)
    .with_error_strategy(ErrorStrategy::Retry(usize::MAX));
  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Retry(usize::MAX)
  ));
}

#[test]
fn test_redis_producer_empty_name() {
  let redis_config = RedisConsumerConfig::default().with_stream("test");
  let producer = RedisProducer::new(redis_config).with_name("".to_string());
  assert_eq!(producer.config.name, Some("".to_string()));
}

#[test]
fn test_redis_consumer_empty_name() {
  let redis_config = RedisProducerConfig::default().with_stream("test");
  let consumer = RedisConsumer::<TestData>::new(redis_config).with_name("".to_string());
  assert_eq!(consumer.config.name, "");
}

#[test]
fn test_redis_producer_config_default_name() {
  let redis_config = RedisConsumerConfig::default().with_stream("test");
  let producer = RedisProducer::new(redis_config);
  assert_eq!(producer.config.name, None);
}

#[test]
fn test_redis_consumer_config_default_name() {
  let redis_config = RedisProducerConfig::default().with_stream("test");
  let consumer = RedisConsumer::<TestData>::new(redis_config);
  assert_eq!(consumer.config.name, "");
}

#[test]
fn test_redis_consumer_config_chaining() {
  let redis_config = RedisProducerConfig::default()
    .with_connection_url("redis://test:6379")
    .with_stream("stream")
    .with_maxlen(100)
    .with_approximate_maxlen(true);

  let consumer = RedisConsumer::<TestData>::new(redis_config)
    .with_name("consumer")
    .with_error_strategy(ErrorStrategy::Skip);

  assert_eq!(consumer.config.name, "consumer");
  assert!(matches!(
    consumer.config.error_strategy,
    ErrorStrategy::Skip
  ));
  assert_eq!(consumer.redis_config().stream, "stream");
}

#[test]
fn test_redis_producer_config_chaining() {
  let redis_config = RedisConsumerConfig::default()
    .with_connection_url("redis://test:6379")
    .with_stream("stream")
    .with_group("group")
    .with_consumer("consumer");

  let producer = RedisProducer::new(redis_config)
    .with_name("producer")
    .with_error_strategy(ErrorStrategy::Retry(3));

  assert_eq!(producer.config.name, Some("producer".to_string()));
  assert!(matches!(
    producer.config.error_strategy,
    ErrorStrategy::Retry(3)
  ));
  assert_eq!(producer.redis_config().stream, "stream");
}
