#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use serde::{Deserialize, Serialize};
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
use streamweave::{
  consumers::kafka::kafka_consumer::{KafkaConsumer, KafkaProducerConfig},
  error::ErrorStrategy,
  pipeline::PipelineBuilder,
  producers::kafka::kafka_producer::{KafkaConsumerConfig, KafkaMessage, KafkaProducer},
  transformers::map::map_transformer::MapTransformer,
};

/// A simple event structure for demonstration
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
  pub id: u32,
  pub message: String,
  pub timestamp: i64,
}

/// Example: Consuming from Kafka and transforming messages
///
/// This demonstrates:
/// - Consuming messages from a Kafka topic with a consumer group
/// - Extracting and parsing message payloads
/// - Error handling for deserialization failures
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
pub async fn consume_from_kafka() -> Result<(), Box<dyn std::error::Error>> {
  println!("📥 Setting up Kafka consumer...");

  // Configure Kafka consumer (which acts as a producer in StreamWeave)
  let kafka_config = KafkaConsumerConfig::default()
    .with_bootstrap_servers("localhost:9092")
    .with_group_id("streamweave-example-group")
    .with_topic("example-topic")
    .with_auto_offset_reset("earliest") // Start from beginning for demo
    .with_enable_auto_commit(true) // Auto-commit offsets
    .with_auto_commit_interval_ms(5000); // Commit every 5 seconds

  let producer = KafkaProducer::new(kafka_config)
    .with_name("kafka-consumer".to_string())
    .with_error_strategy(ErrorStrategy::Skip); // Skip messages that fail to deserialize

  // Transform Kafka messages to extract JSON payloads
  let transformer = MapTransformer::new(|msg: KafkaMessage| -> Result<Event, String> {
    // Try to deserialize the message payload as JSON
    match serde_json::from_slice::<Event>(&msg.payload) {
      Ok(event) => {
        println!(
          "  ✓ Received message: topic={}, partition={}, offset={}, id={}, message={}",
          msg.topic, msg.partition, msg.offset, event.id, event.message
        );
        Ok(event)
      }
      Err(e) => {
        eprintln!(
          "  ✗ Failed to deserialize message at offset {}: {}",
          msg.offset, e
        );
        Err(format!("Deserialization error: {}", e))
      }
    }
  })
  .with_error_strategy(ErrorStrategy::Skip);

  // Use VecConsumer to collect events (ConsoleConsumer requires Display, but we have Result)
  use streamweave::consumers::vec::vec_consumer::VecConsumer;

  println!("🚀 Starting pipeline to consume from Kafka...");
  println!("   Topic: example-topic");
  println!("   Consumer Group: streamweave-example-group");
  println!("   Press Ctrl+C to stop\n");

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(VecConsumer::new());

  // Run the pipeline (this will block until the stream ends or is interrupted)
  pipeline.run().await?;

  Ok(())
}

/// Example: Producing to Kafka with batching
///
/// This demonstrates:
/// - Creating events and sending them to Kafka
/// - Producer batching configuration for efficiency
/// - Error handling for send failures
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
pub async fn produce_to_kafka() -> Result<(), Box<dyn std::error::Error>> {
  println!("📤 Setting up Kafka producer...");

  // Create sample events
  use std::time::{SystemTime, UNIX_EPOCH};
  use streamweave::producers::vec::vec_producer::VecProducer;

  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs() as i64;

  let events = vec![
    Event {
      id: 1,
      message: "First event".to_string(),
      timestamp: now,
    },
    Event {
      id: 2,
      message: "Second event".to_string(),
      timestamp: now + 1,
    },
    Event {
      id: 3,
      message: "Third event".to_string(),
      timestamp: now + 2,
    },
    Event {
      id: 4,
      message: "Fourth event".to_string(),
      timestamp: now + 3,
    },
    Event {
      id: 5,
      message: "Fifth event".to_string(),
      timestamp: now + 4,
    },
  ];

  // Configure Kafka producer (which acts as a consumer in StreamWeave)
  let kafka_config = KafkaProducerConfig::default()
    .with_bootstrap_servers("localhost:9092")
    .with_topic("example-topic")
    .with_client_id("streamweave-example-producer")
    .with_acks("all") // Wait for all replicas to acknowledge
    .with_retries(3) // Retry up to 3 times on failure
    .with_batch_size(16384) // 16KB batch size
    .with_linger_ms(10) // Wait up to 10ms to fill batch
    .with_compression_type("gzip"); // Compress messages

  let consumer = KafkaConsumer::new(kafka_config)
    .with_name("kafka-producer".to_string())
    .with_error_strategy(ErrorStrategy::Retry(3)); // Retry failed sends

  println!("🚀 Starting pipeline to produce to Kafka...");
  println!("   Topic: example-topic");
  println!("   Batch size: 16KB");
  println!("   Compression: gzip");
  println!("   Sending {} events...\n", events.len());

  // Add identity transformer to pass through events unchanged
  let identity_transformer =
    MapTransformer::new(|event: Event| -> Result<Event, String> { Ok(event) });

  let pipeline = PipelineBuilder::new()
    .producer(VecProducer::new(events))
    .transformer(identity_transformer)
    .consumer(consumer);

  // Run the pipeline
  pipeline.run().await?;

  println!("\n✅ All events sent successfully!");
  Ok(())
}

/// Example: Full round-trip - consume, transform, and produce back
///
/// This demonstrates:
/// - Reading from one topic
/// - Transforming messages
/// - Writing to another topic
/// - Offset management through consumer groups
#[cfg(all(not(target_arch = "wasm32"), feature = "kafka"))]
pub async fn round_trip_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("🔄 Setting up round-trip Kafka pipeline...");

  // Consumer configuration
  let consumer_config = KafkaConsumerConfig::default()
    .with_bootstrap_servers("localhost:9092")
    .with_group_id("streamweave-roundtrip-group")
    .with_topic("input-topic")
    .with_auto_offset_reset("earliest");

  let kafka_producer = KafkaProducer::new(consumer_config)
    .with_name("kafka-input".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Transform: parse JSON and add processing metadata
  let transformer = MapTransformer::new(|msg: KafkaMessage| -> Result<Event, String> {
    match serde_json::from_slice::<Event>(&msg.payload) {
      Ok(mut event) => {
        // Add processing indicator
        event.message = format!("[PROCESSED] {}", event.message);
        println!("  Processing: offset={}, event_id={}", msg.offset, event.id);
        Ok(event)
      }
      Err(e) => Err(format!("Parse error: {}", e)),
    }
  });

  // Producer configuration
  let producer_config = KafkaProducerConfig::default()
    .with_bootstrap_servers("localhost:9092")
    .with_topic("output-topic")
    .with_batch_size(32768) // 32KB for output
    .with_linger_ms(50); // Longer linger for better batching

  let kafka_consumer = KafkaConsumer::new(producer_config)
    .with_name("kafka-output".to_string())
    .with_error_strategy(ErrorStrategy::Retry(5));

  println!("🚀 Starting round-trip pipeline...");
  println!("   Input topic: input-topic");
  println!("   Output topic: output-topic");
  println!("   Consumer Group: streamweave-roundtrip-group\n");

  let pipeline = PipelineBuilder::new()
    .producer(kafka_producer)
    .transformer(transformer)
    .consumer(kafka_consumer);

  pipeline.run().await?;

  println!("\n✅ Round-trip processing completed!");
  Ok(())
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "kafka")))]
#[allow(dead_code)]
pub async fn consume_from_kafka() -> Result<(), Box<dyn std::error::Error>> {
  Err("Kafka feature is not enabled. Build with --features kafka".into())
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "kafka")))]
#[allow(dead_code)]
pub async fn produce_to_kafka() -> Result<(), Box<dyn std::error::Error>> {
  Err("Kafka feature is not enabled. Build with --features kafka".into())
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "kafka")))]
#[allow(dead_code)]
pub async fn round_trip_example() -> Result<(), Box<dyn std::error::Error>> {
  Err("Kafka feature is not enabled. Build with --features kafka".into())
}
