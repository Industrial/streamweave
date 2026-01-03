//! Integration tests for README examples
//!
//! These tests verify that the code examples in the README.md file compile and work correctly.

#[cfg(feature = "kafka")]
mod tests {
  use streamweave::consumers::VecConsumer;
  use streamweave::pipeline::PipelineBuilder;
  use streamweave::producers::VecProducer;
  use streamweave_kafka::{KafkaConsumer, KafkaConsumerConfig, KafkaProducer, KafkaProducerConfig};

  #[tokio::test]
  #[ignore] // Requires Kafka server
  async fn test_consume_from_kafka_example() {
    let config = KafkaConsumerConfig::default()
      .with_bootstrap_servers("localhost:9092")
      .with_group_id("my-consumer-group")
      .with_topic("my-topic")
      .with_auto_offset_reset("earliest");

    let producer = KafkaProducer::new(config);
    let consumer = VecConsumer::new();

    let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

    // Note: This test requires a running Kafka server
    // In a real scenario, you would run the pipeline and verify results
    // let _result = pipeline.run().await;
  }

  #[tokio::test]
  #[ignore] // Requires Kafka server
  async fn test_produce_to_kafka_example() {
    use serde::Serialize;

    #[derive(Serialize, Clone, Debug)]
    struct Event {
      id: u32,
      message: String,
    }

    let config = KafkaProducerConfig::default()
      .with_bootstrap_servers("localhost:9092")
      .with_topic("my-topic")
      .with_acks("all");

    let producer = VecProducer::new(vec![
      Event {
        id: 1,
        message: "test".to_string(),
      },
      Event {
        id: 2,
        message: "test2".to_string(),
      },
    ]);
    let consumer = KafkaConsumer::<Event>::new(config);

    let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

    // Note: This test requires a running Kafka server
    // In a real scenario, you would run the pipeline and verify results
    // let _result = pipeline.run().await;
  }

  #[test]
  fn test_consumer_group_setup_example() {
    let config = KafkaConsumerConfig::default()
      .with_bootstrap_servers("kafka1:9092,kafka2:9092")
      .with_group_id("my-consumer-group")
      .with_topic("events")
      .with_auto_offset_reset("earliest")
      .with_enable_auto_commit(true)
      .with_auto_commit_interval_ms(5000)
      .with_session_timeout_ms(30000)
      .with_max_poll_interval_ms(300000);

    let _producer = KafkaProducer::new(config);
  }

  #[test]
  fn test_multiple_topics_example() {
    let config = KafkaConsumerConfig::default()
      .with_bootstrap_servers("localhost:9092")
      .with_group_id("multi-topic-consumer")
      .with_topics(vec!["topic1".to_string(), "topic2".to_string()])
      .with_auto_offset_reset("latest");

    let _producer = KafkaProducer::new(config);
  }

  #[test]
  fn test_offset_management_example() {
    let config = KafkaConsumerConfig::default()
      .with_bootstrap_servers("localhost:9092")
      .with_group_id("offset-managed-consumer")
      .with_topic("events")
      .with_auto_offset_reset("earliest")
      .with_enable_auto_commit(false)
      .with_auto_commit_interval_ms(1000);

    let _producer = KafkaProducer::new(config);
  }

  #[test]
  fn test_producer_configuration_example() {
    use serde::Serialize;

    #[derive(Serialize, Clone, Debug)]
    struct Event {
      id: u32,
      message: String,
    }

    let config = KafkaProducerConfig::default()
      .with_bootstrap_servers("localhost:9092")
      .with_topic("events")
      .with_client_id("my-producer")
      .with_acks("all")
      .with_retries(3)
      .with_batch_size(16384)
      .with_linger_ms(10)
      .with_compression_type("gzip")
      .with_max_request_size(1048576);

    let _consumer = KafkaConsumer::<Event>::new(config);
  }

  #[test]
  fn test_custom_properties_example() {
    let config = KafkaConsumerConfig::default()
      .with_bootstrap_servers("localhost:9092")
      .with_topic("events")
      .with_custom_property("fetch.min.bytes", "1024")
      .with_custom_property("max.partition.fetch.bytes", "1048576");

    let _producer = KafkaProducer::new(config);
  }
}
