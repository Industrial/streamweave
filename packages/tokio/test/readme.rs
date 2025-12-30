//! Integration tests for README examples
//!
//! These tests verify that the code examples in the README.md file work correctly.

use futures::StreamExt;
use streamweave_pipeline::PipelineBuilder;
use streamweave_tokio::{ChannelConsumer, ChannelProducer};
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;
use streamweave_vec::VecProducer;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_readme_read_from_channel() {
  // Example: Read from Channel
  // ```rust
  // use streamweave_tokio::ChannelProducer;
  // use streamweave_pipeline::PipelineBuilder;
  // use tokio::sync::mpsc;
  //
  // let (sender, receiver) = mpsc::channel(100);
  // let producer = ChannelProducer::new(receiver);
  //
  // let pipeline = PipelineBuilder::new()
  //     .producer(producer)
  //     .consumer(/* process items */);
  //
  // pipeline.run().await?;
  // ```

  let (sender, receiver) = mpsc::channel(100);

  // Send some items to the channel
  tokio::spawn(async move {
    for i in 0..5 {
      sender.send(i).await.unwrap();
    }
  });

  let producer = ChannelProducer::new(receiver);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

  let ((), mut consumer) = pipeline.run().await.unwrap();

  let collected = consumer.collected;
  assert_eq!(collected.len(), 5);
  assert_eq!(collected, vec![0, 1, 2, 3, 4]);
}

#[tokio::test]
async fn test_readme_write_to_channel() {
  // Example: Write to Channel
  // ```rust
  // use streamweave_tokio::ChannelConsumer;
  // use streamweave_pipeline::PipelineBuilder;
  // use tokio::sync::mpsc;
  //
  // let (sender, receiver) = mpsc::channel(100);
  // let consumer = ChannelConsumer::new(sender);
  //
  // let pipeline = PipelineBuilder::new()
  //     .producer(/* produce items */)
  //     .consumer(consumer);
  //
  // pipeline.run().await?;
  // ```

  let (sender, mut receiver) = mpsc::channel(100);
  let consumer = ChannelConsumer::new(sender);

  let producer = VecProducer::new(vec![10, 20, 30]);

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

  let handle = tokio::spawn(async move {
    let mut received = Vec::new();
    while let Some(item) = receiver.recv().await {
      received.push(item);
    }
    received
  });

  pipeline.run().await.unwrap();

  let received = handle.await.unwrap();
  assert_eq!(received, vec![10, 20, 30]);
}

#[tokio::test]
async fn test_readme_channel_bridge() {
  // Example: Channel Bridge
  // ```rust
  // use streamweave_tokio::{ChannelProducer, ChannelConsumer};
  // use streamweave_pipeline::PipelineBuilder;
  // use tokio::sync::mpsc;
  //
  // let (input_sender, input_receiver) = mpsc::channel(100);
  // let (output_sender, output_receiver) = mpsc::channel(100);
  //
  // let producer = ChannelProducer::new(input_receiver);
  // let consumer = ChannelConsumer::new(output_sender);
  //
  // let pipeline = PipelineBuilder::new()
  //     .producer(producer)
  //     .transformer(|x: i32| x * 2)
  //     .consumer(consumer);
  //
  // pipeline.run().await?;
  // ```

  let (input_sender, input_receiver) = mpsc::channel(100);
  let (output_sender, mut output_receiver) = mpsc::channel(100);

  let producer = ChannelProducer::new(input_receiver);
  let consumer = ChannelConsumer::new(output_sender);

  // Send items to input channel
  tokio::spawn(async move {
    for i in 1..=5 {
      input_sender.send(i).await.unwrap();
    }
  });

  let transformer = MapTransformer::new(|x: i32| x * 2);

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

  let handle = tokio::spawn(async move {
    let mut received = Vec::new();
    while let Some(item) = output_receiver.recv().await {
      received.push(item);
    }
    received
  });

  pipeline.run().await.unwrap();

  let received = handle.await.unwrap();
  assert_eq!(received, vec![2, 4, 6, 8, 10]);
}

#[tokio::test]
async fn test_readme_async_integration() {
  // Example: Async Integration
  // ```rust
  // use streamweave_tokio::ChannelProducer;
  // use streamweave_pipeline::PipelineBuilder;
  // use tokio::sync::mpsc;
  //
  // let (sender, receiver) = mpsc::channel(100);
  //
  // // Spawn async task that sends to channel
  // tokio::spawn(async move {
  //     for i in 0..10 {
  //         sender.send(i).await.unwrap();
  //     }
  // });
  //
  // let producer = ChannelProducer::new(receiver);
  //
  // let pipeline = PipelineBuilder::new()
  //     .producer(producer)
  //     .consumer(/* process items */);
  //
  // pipeline.run().await?;
  // ```

  let (sender, receiver) = mpsc::channel(100);

  // Spawn async task that sends to channel
  tokio::spawn(async move {
    for i in 0..10 {
      sender.send(i).await.unwrap();
    }
  });

  let producer = ChannelProducer::new(receiver);
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);

  let ((), mut consumer) = pipeline.run().await.unwrap();

  let collected = consumer.collected;
  assert_eq!(collected.len(), 10);
  assert_eq!(collected, (0..10).collect::<Vec<i32>>());
}

#[tokio::test]
async fn test_readme_error_handling() {
  // Example: Error Handling
  // ```rust
  // use streamweave_tokio::ChannelProducer;
  // use streamweave_error::ErrorStrategy;
  // use tokio::sync::mpsc;
  //
  // let (_, receiver) = mpsc::channel(100);
  // let producer = ChannelProducer::new(receiver)
  //     .with_error_strategy(ErrorStrategy::Skip);
  // ```

  use streamweave_error::ErrorStrategy;

  let (_, receiver) = mpsc::channel::<i32>(100);
  let producer = ChannelProducer::new(receiver).with_error_strategy(ErrorStrategy::Skip);

  assert_eq!(
    producer.config().error_strategy(),
    &ErrorStrategy::<i32>::Skip
  );
}
