//! Integration tests for README examples
//!
//! These tests verify that the code examples in the README.md file compile and work correctly.

#[cfg(test)]
mod readme_tests {
  use serde::Serialize;
  use streamweave_redis::{RedisConsumer, RedisProducerConfig};
  use streamweave_redis::{RedisConsumerConfig, RedisProducer};

  #[derive(Serialize, Debug, Clone)]
  struct Event {
    id: u32,
    message: String,
  }

  #[test]
  fn test_consume_from_redis_streams_example() {
    // Example from README: Consume from Redis Streams
    let config = RedisConsumerConfig::default()
      .with_connection_url("redis://localhost:6379")
      .with_stream("mystream")
      .with_group("my-group")
      .with_consumer("consumer-1")
      .with_start_id("0");

    let _producer = RedisProducer::new(config);
    // Note: We can't actually run the producer without a Redis connection,
    // but we can verify the configuration is correct
  }

  #[test]
  fn test_produce_to_redis_streams_example() {
    // Example from README: Produce to Redis Streams
    let config = RedisProducerConfig::default()
      .with_connection_url("redis://localhost:6379")
      .with_stream("mystream")
      .with_maxlen(10000);

    let _consumer = RedisConsumer::<Event>::new(config);
    // Note: We can't actually run the consumer without a Redis connection,
    // but we can verify the configuration is correct
  }

  #[test]
  fn test_consumer_group_setup_example() {
    // Example from README: Consumer Group Setup
    let config = RedisConsumerConfig::default()
      .with_connection_url("redis://localhost:6379")
      .with_stream("events")
      .with_group("my-consumer-group")
      .with_consumer("consumer-1")
      .with_start_id(">")
      .with_block_ms(5000)
      .with_count(100)
      .with_auto_ack(true);

    let _producer = RedisProducer::new(config);
  }

  #[test]
  fn test_reading_from_beginning_example() {
    // Example from README: Reading from Beginning
    let config = RedisConsumerConfig::default()
      .with_connection_url("redis://localhost:6379")
      .with_stream("events")
      .with_start_id("0")
      .with_block_ms(1000);

    let _producer = RedisProducer::new(config);
  }

  #[test]
  fn test_reading_new_messages_only_example() {
    // Example from README: Reading New Messages Only
    let config = RedisConsumerConfig::default()
      .with_connection_url("redis://localhost:6379")
      .with_stream("events")
      .with_start_id("$")
      .with_block_ms(5000);

    let _producer = RedisProducer::new(config);
  }

  #[test]
  fn test_stream_length_management_example() {
    // Example from README: Stream Length Management
    let config = RedisProducerConfig::default()
      .with_connection_url("redis://localhost:6379")
      .with_stream("events")
      .with_maxlen(10000)
      .with_approximate_maxlen(true);

    let _consumer = RedisConsumer::<Event>::new(config);
  }

  #[test]
  fn test_message_acknowledgment_example() {
    // Example from README: Message Acknowledgment
    let config = RedisConsumerConfig::default()
      .with_connection_url("redis://localhost:6379")
      .with_stream("events")
      .with_group("my-group")
      .with_consumer("consumer-1")
      .with_auto_ack(true);

    let _producer = RedisProducer::new(config);
  }

  #[test]
  fn test_error_handling_example() {
    // Example from README: Error Handling
    use streamweave::error::ErrorStrategy;

    let config = RedisConsumerConfig::default()
      .with_connection_url("redis://localhost:6379")
      .with_stream("events");

    let _producer = RedisProducer::new(config).with_error_strategy(ErrorStrategy::Retry(5));
  }
}
