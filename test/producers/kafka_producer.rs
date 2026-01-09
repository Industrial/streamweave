use streamweave::producers::{KafkaConsumerConfig, KafkaProducer};

#[test]
fn test_kafka_consumer_config_default() {
  let config = KafkaConsumerConfig::default();
  assert_eq!(config.bootstrap_servers, "localhost:9092");
  assert!(config.topics.is_empty());
  assert_eq!(config.auto_offset_reset, "earliest");
  assert!(config.enable_auto_commit);
}

#[test]
fn test_kafka_consumer_config_builder() {
  let config = KafkaConsumerConfig::default()
    .with_bootstrap_servers("kafka:9092")
    .with_group_id("test-group")
    .with_topic("test-topic")
    .with_auto_offset_reset("latest")
    .with_enable_auto_commit(false);

  assert_eq!(config.bootstrap_servers, "kafka:9092");
  assert_eq!(config.group_id, Some("test-group".to_string()));
  assert_eq!(config.topics.len(), 1);
  assert_eq!(config.topics[0], "test-topic");
  assert_eq!(config.auto_offset_reset, "latest");
  assert!(!config.enable_auto_commit);
}

#[test]
fn test_kafka_producer_new() {
  let kafka_config = KafkaConsumerConfig::default().with_topic("test-topic");
  let producer = KafkaProducer::new(kafka_config);
  assert_eq!(producer.kafka_config().topics.len(), 1);
}

#[test]
fn test_kafka_consumer_config_with_topics() {
  let config =
    KafkaConsumerConfig::default().with_topics(vec!["topic1".to_string(), "topic2".to_string()]);
  assert_eq!(config.topics.len(), 2);
  assert_eq!(config.topics[0], "topic1");
  assert_eq!(config.topics[1], "topic2");
}

#[test]
fn test_kafka_consumer_config_with_auto_commit_interval() {
  let config = KafkaConsumerConfig::default().with_auto_commit_interval_ms(10000);
  assert_eq!(config.auto_commit_interval_ms, 10000);
}

#[test]
fn test_kafka_consumer_config_with_custom_property() {
  let config = KafkaConsumerConfig::default()
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
fn test_kafka_producer_builder() {
  let kafka_config = KafkaConsumerConfig::default()
    .with_topic("test-topic")
    .with_group_id("test-group");
  let producer = KafkaProducer::new(kafka_config)
    .with_name("my_producer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);
  assert_eq!(producer.config().name, Some("my_producer".to_string()));
  assert!(matches!(
    producer.config().error_strategy,
    ErrorStrategy::Skip
  ));
}

#[test]
fn test_kafka_producer_clone() {
  let kafka_config = KafkaConsumerConfig::default().with_topic("test-topic");
  let producer = KafkaProducer::new(kafka_config).with_name("test".to_string());
  let cloned = producer.clone();
  assert_eq!(cloned.config().name, producer.config().name);
  assert_eq!(cloned.kafka_config().topics, producer.kafka_config().topics);
}

#[test]
fn test_kafka_producer_config_methods() {
  use streamweave::Producer;
  use streamweave::error::{ComponentInfo, ErrorContext, ErrorStrategy, StreamError};

  let mut producer = KafkaProducer::new(KafkaConsumerConfig::default().with_topic("test-topic"));
  let config = producer.config().clone();
  assert_eq!(config.name, None);

  let config_mut = producer.config_mut();
  config_mut.name = Some("test".to_string());
  assert_eq!(producer.config().name, Some("test".to_string()));

  let new_config = streamweave::ProducerConfig::default();
  producer.set_config(new_config.clone());
  assert_eq!(producer.config().name, new_config.name);
}

#[test]
fn test_kafka_producer_handle_error() {
  use streamweave::Producer;
  use streamweave::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
  use streamweave::producers::KafkaMessage;

  let producer = KafkaProducer::new(KafkaConsumerConfig::default().with_topic("test-topic"))
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
  assert_eq!(producer.handle_error(&error), ErrorAction::Skip);

  let producer_stop = KafkaProducer::new(KafkaConsumerConfig::default().with_topic("test-topic"))
    .with_error_strategy(ErrorStrategy::Stop);
  assert_eq!(producer_stop.handle_error(&error), ErrorAction::Stop);

  let producer_retry = KafkaProducer::new(KafkaConsumerConfig::default().with_topic("test-topic"))
    .with_error_strategy(ErrorStrategy::Retry(3));
  let mut error_retry = error.clone();
  error_retry.retries = 0;
  assert_eq!(
    producer_retry.handle_error(&error_retry),
    ErrorAction::Retry
  );

  let mut error_retry_max = error.clone();
  error_retry_max.retries = 3;
  assert_eq!(
    producer_retry.handle_error(&error_retry_max),
    ErrorAction::Stop
  );
}

#[test]
fn test_kafka_producer_create_error_context() {
  use streamweave::Producer;
  use streamweave::producers::KafkaMessage;

  let producer = KafkaProducer::new(KafkaConsumerConfig::default().with_topic("test-topic"))
    .with_name("test_producer".to_string());
  let ctx = producer.create_error_context(Some(KafkaMessage {
    topic: "test".to_string(),
    partition: 0,
    offset: 0,
    key: None,
    payload: vec![],
    timestamp: None,
    headers: std::collections::HashMap::new(),
  }));
  assert_eq!(ctx.component_name, "test_producer");

  let ctx_no_item = producer.create_error_context(None);
  assert_eq!(ctx_no_item.component_name, "test_producer");
  assert_eq!(ctx_no_item.item, None);
}

#[test]
fn test_kafka_producer_component_info() {
  use streamweave::Producer;

  let producer = KafkaProducer::new(KafkaConsumerConfig::default().with_topic("test-topic"))
    .with_name("my_producer".to_string());
  let info = producer.component_info();
  assert_eq!(info.name, "my_producer");
  assert_eq!(info.type_name, std::any::type_name::<KafkaProducer>());
}

#[test]
fn test_kafka_producer_default_name() {
  use streamweave::Producer;

  let producer = KafkaProducer::new(KafkaConsumerConfig::default().with_topic("test-topic"));
  let info = producer.component_info();
  assert_eq!(info.name, "kafka_producer");
}
