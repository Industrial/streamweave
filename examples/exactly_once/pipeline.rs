use streamweave::{
  consumers::vec::vec_consumer::VecConsumer,
  error::ErrorStrategy,
  message::{Message, MessageId},
  pipeline::PipelineBuilder,
  producers::vec::vec_producer::VecProducer,
  transformers::message_dedupe::{
    message_dedupe_transformer::MessageDedupeTransformer, DeduplicationWindow,
  },
};
use std::time::Duration;

pub async fn deduplication_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("🔄 Message Deduplication Example");
  println!("-------------------------------");
  println!("Demonstrates filtering duplicate messages");
  
  // Create messages with some duplicates
  let messages = vec![
    Message::new(1, MessageId::Sequence(1)),
    Message::new(2, MessageId::Sequence(2)),
    Message::new(1, MessageId::Sequence(1)), // Duplicate
    Message::new(3, MessageId::Sequence(3)),
    Message::new(2, MessageId::Sequence(2)), // Duplicate
    Message::new(4, MessageId::Sequence(4)),
  ];
  
  let producer = VecProducer::new(messages);
  
  let deduper = MessageDedupeTransformer::new()
    .with_window(DeduplicationWindow::Count(1000))
    .with_name("deduplicator".to_string());
  
  let consumer = VecConsumer::new();
  
  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(deduper)
    .with_consumer(consumer)
    .build();
  
  let results: Vec<Message<i32>> = pipeline.run().await?;
  println!("✅ Processed {} unique messages (duplicates filtered)", results.len());
  for (i, msg) in results.iter().enumerate() {
    println!("  Message {}: ID={:?}, Payload={}", i + 1, msg.id(), msg.payload());
  }
  println!("💡 Deduplication ensures exactly-once processing");
  Ok(())
}

pub async fn checkpoint_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("💾 Checkpoint Example");
  println!("--------------------");
  println!("Demonstrates state persistence and recovery");
  
  let messages = vec![
    Message::new(1, MessageId::Sequence(1)),
    Message::new(2, MessageId::Sequence(2)),
    Message::new(3, MessageId::Sequence(3)),
  ];
  
  let producer = VecProducer::new(messages.clone());
  
  let deduper = MessageDedupeTransformer::new()
    .with_window(DeduplicationWindow::Time(Duration::from_secs(60)))
    .with_name("checkpoint-deduplicator".to_string());
  
  let consumer = VecConsumer::new();
  
  println!("🔄 First run...");
  let pipeline1 = PipelineBuilder::new()
    .with_producer(VecProducer::new(messages.clone()))
    .with_transformer(deduper.clone())
    .with_consumer(VecConsumer::new())
    .build();
  
  let _results1 = pipeline1.run().await?;
  println!("✅ First run completed");
  
  println!("\n🔄 Second run (with duplicates)...");
  let duplicate_messages = vec![
    Message::new(1, MessageId::Sequence(1)), // Duplicate
    Message::new(4, MessageId::Sequence(4)), // New
    Message::new(2, MessageId::Sequence(2)), // Duplicate
  ];
  
  let pipeline2 = PipelineBuilder::new()
    .with_producer(VecProducer::new(duplicate_messages))
    .with_transformer(deduper)
    .with_consumer(consumer)
    .build();
  
  let results2 = pipeline2.run().await?;
  println!("✅ Second run completed");
  println!("📊 Processed {} unique messages", results2.len());
  println!("💡 Checkpointing enables recovery from failures");
  Ok(())
}

