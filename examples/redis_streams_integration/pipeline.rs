#[cfg(feature = "redis")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "redis")]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(feature = "redis")]
use streamweave::{
  consumers::redis::redis_consumer::{RedisConsumer, RedisProducerConfig},
  error::ErrorStrategy,
  pipeline::PipelineBuilder,
  producers::redis::redis_producer::{RedisConsumerConfig, RedisMessage, RedisProducer},
  producers::vec::vec_producer::VecProducer,
  transformers::map::map_transformer::MapTransformer,
};

/// A simple event structure for demonstration
#[cfg(feature = "redis")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
  pub id: u32,
  pub message: String,
  pub timestamp: i64,
}

/// Example: Consuming from Redis Streams with consumer groups
///
/// This demonstrates:
/// - Consuming messages from a Redis stream using XREAD
/// - Using consumer groups for distributed processing
/// - Message acknowledgment patterns
/// - Error handling for deserialization failures
#[cfg(feature = "redis")]
pub async fn consume_from_redis() -> Result<(), Box<dyn std::error::Error>> {
  println!("📥 Setting up Redis Streams consumer...");

  // Configure Redis Streams consumer (which acts as a producer in StreamWeave)
  let redis_config = RedisConsumerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("example-stream")
    .with_group("streamweave-example-group") // Consumer group for distributed processing
    .with_consumer("consumer-1") // Consumer name within the group
    .with_start_id(">") // Read new messages (use "0" to read from beginning)
    .with_block_ms(1000) // Block for 1 second waiting for new messages
    .with_count(10) // Read up to 10 messages per call
    .with_auto_ack(true); // Automatically acknowledge messages

  let producer = RedisProducer::new(redis_config)
    .with_name("redis-consumer".to_string())
    .with_error_strategy(ErrorStrategy::Skip); // Skip messages that fail to deserialize

  // Transform Redis Streams messages to extract JSON payloads
  let transformer = MapTransformer::new(|msg: RedisMessage| -> Result<Event, String> {
    // Try to deserialize the message fields as JSON
    // Redis Streams stores fields as key-value pairs, we look for a "data" field
    if let Some(data_str) = msg.fields.get("data") {
      match serde_json::from_str::<Event>(data_str) {
        Ok(event) => {
          println!(
            "  ✓ Received message: stream={}, id={}, event_id={}, message={}",
            msg.stream, msg.id, event.id, event.message
          );
          Ok(event)
        }
        Err(e) => {
          eprintln!("  ✗ Failed to deserialize message at ID {}: {}", msg.id, e);
          Err(format!("Deserialization error: {}", e))
        }
      }
    } else {
      // If no "data" field, try to reconstruct from all fields
      let json_value = serde_json::json!(msg.fields);
      match serde_json::from_value::<Event>(json_value) {
        Ok(event) => {
          println!(
            "  ✓ Received message: stream={}, id={}, event_id={}",
            msg.stream, msg.id, event.id
          );
          Ok(event)
        }
        Err(e) => {
          eprintln!("  ✗ Failed to deserialize message at ID {}: {}", msg.id, e);
          Err(format!("Deserialization error: {}", e))
        }
      }
    }
  })
  .with_error_strategy(ErrorStrategy::Skip);

  // Use VecConsumer to collect events (ConsoleConsumer requires Display, but we have Result)
  use streamweave::consumers::vec::vec_consumer::VecConsumer;

  println!("🚀 Starting pipeline to consume from Redis Streams...");
  println!("   Stream: example-stream");
  println!("   Consumer Group: streamweave-example-group");
  println!("   Consumer: consumer-1");
  println!("   Press Ctrl+C to stop\n");

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(VecConsumer::new());

  // Run the pipeline (this will block until the stream ends or is interrupted)
  pipeline.run().await?;

  Ok(())
}

/// Example: Producing to Redis Streams
///
/// This demonstrates:
/// - Creating events and sending them to Redis Streams
/// - Serializing events to JSON
/// - Error handling for send failures
#[cfg(feature = "redis")]
pub async fn produce_to_redis() -> Result<(), Box<dyn std::error::Error>> {
  println!("📤 Setting up Redis Streams producer...");

  // Create sample events

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

  // Configure Redis Streams producer (which acts as a consumer in StreamWeave)
  let redis_config = RedisProducerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("example-stream")
    .with_maxlen(1000) // Keep only the last 1000 messages
    .with_approximate_maxlen(true); // Use approximate trimming for efficiency

  let consumer = RedisConsumer::new(redis_config)
    .with_name("redis-producer".to_string())
    .with_error_strategy(ErrorStrategy::Retry(3)); // Retry failed sends

  println!("🚀 Starting pipeline to produce to Redis Streams...");
  println!("   Stream: example-stream");
  println!("   Max length: 1000 messages (approximate)");
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
/// - Reading from one stream
/// - Transforming messages
/// - Writing to another stream
/// - Consumer group offset management
#[cfg(feature = "redis")]
pub async fn round_trip_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("🔄 Setting up round-trip Redis Streams pipeline...");

  // Consumer configuration (reads from input stream)
  let consumer_config = RedisConsumerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("input-stream")
    .with_group("streamweave-roundtrip-group")
    .with_consumer("roundtrip-consumer-1")
    .with_start_id(">") // Read new messages
    .with_block_ms(1000)
    .with_auto_ack(true);

  let redis_producer = RedisProducer::new(consumer_config)
    .with_name("redis-input".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Transform: parse JSON and add processing metadata
  let transformer = MapTransformer::new(|msg: RedisMessage| -> Result<Event, String> {
    // Extract event from message fields
    if let Some(data_str) = msg.fields.get("data") {
      match serde_json::from_str::<Event>(data_str) {
        Ok(mut event) => {
          // Add processing indicator
          event.message = format!("[PROCESSED] {}", event.message);
          println!("  Processing: id={}, event_id={}", msg.id, event.id);
          Ok(event)
        }
        Err(e) => Err(format!("Parse error: {}", e)),
      }
    } else {
      Err("No 'data' field found".to_string())
    }
  })
  .with_error_strategy(ErrorStrategy::Skip);

  // Producer configuration (writes to output stream)
  let producer_config = RedisProducerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("output-stream")
    .with_maxlen(5000) // Larger buffer for output
    .with_approximate_maxlen(true);

  let redis_consumer = RedisConsumer::new(producer_config)
    .with_name("redis-output".to_string())
    .with_error_strategy(ErrorStrategy::Retry(5));

  println!("🚀 Starting round-trip pipeline...");
  println!("   Input stream: input-stream");
  println!("   Output stream: output-stream");
  println!("   Consumer Group: streamweave-roundtrip-group\n");

  let pipeline = PipelineBuilder::new()
    .producer(redis_producer)
    .transformer(transformer)
    .consumer(redis_consumer);

  pipeline.run().await?;

  println!("\n✅ Round-trip processing completed!");
  Ok(())
}

#[cfg(not(feature = "redis"))]
#[allow(dead_code)]
pub async fn consume_from_redis() -> Result<(), Box<dyn std::error::Error>> {
  Err("Redis Streams feature is not enabled. Build with --features redis".into())
}

#[cfg(not(feature = "redis"))]
#[allow(dead_code)]
pub async fn produce_to_redis() -> Result<(), Box<dyn std::error::Error>> {
  Err("Redis Streams feature is not enabled. Build with --features redis".into())
}

#[cfg(not(feature = "redis"))]
#[allow(dead_code)]
pub async fn round_trip_example() -> Result<(), Box<dyn std::error::Error>> {
  Err("Redis Streams feature is not enabled. Build with --features redis".into())
}
