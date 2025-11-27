use streamweave::{
  consumers::vec::vec_consumer::VecConsumer, error::ErrorStrategy, pipeline::PipelineBuilder,
  producers::range::range_producer::RangeProducer, stateful_transformer::StatefulTransformer,
  transformers::moving_average::MovingAverageTransformer,
  transformers::running_sum::RunningSumTransformer,
};

/// Example: Running Sum Transformer
///
/// This demonstrates:
/// - Maintaining cumulative state across stream items
/// - Running sum calculations
/// - State persistence across transformations
/// - State inspection and reset
pub async fn running_sum_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📊 Setting up running sum example...");

  // Create a producer that generates numbers 1-10
  let producer = RangeProducer::new(1, 11, 1)
    .with_name("number-generator".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Create running sum transformer
  // State starts at 0 (default) and accumulates each value
  let transformer = RunningSumTransformer::<i32>::new()
    .with_name("running-sum".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Collect results
  let consumer = VecConsumer::new();

  println!("🔄 Processing numbers 1-10 with running sum...");
  println!("   Input:  [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]");
  println!("   Output: [1, 3, 6, 10, 15, 21, 28, 36, 45, 55]");
  println!();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer.clone())
    .consumer(consumer);

  let (_, result_consumer) = pipeline.run().await?;
  let results = result_consumer.into_vec();

  println!("✅ Running sum completed!");
  println!("📊 Results:");
  for (i, sum) in results.iter().enumerate() {
    println!("  Item {}: Running sum = {}", i + 1, sum);
  }

  // Demonstrate state inspection
  println!("\n🔍 State Inspection:");
  if let Ok(Some(state)) = transformer.state() {
    println!("   Current state (final sum): {}", state);
  }
  println!("   State is initialized: {}", transformer.has_state());

  // Demonstrate state reset
  println!("\n🔄 Resetting state...");
  transformer.reset_state()?;
  println!("   State after reset: {:?}", transformer.state()?);
  println!("   State is initialized: {}", transformer.has_state());

  // Run again with reset state (should start from 0 again)
  println!("\n🔄 Running again with reset state...");
  let producer2 = RangeProducer::new(1, 6, 1);
  let transformer2 = RunningSumTransformer::<i32>::new();
  let consumer2 = VecConsumer::new();

  let pipeline2 = PipelineBuilder::new()
    .producer(producer2)
    .transformer(transformer2)
    .consumer(consumer2);

  let (_, result_consumer2) = pipeline2.run().await?;
  let results2 = result_consumer2.into_vec();
  println!("   Results after reset: {:?}", results2);
  println!("   ✅ State correctly reset to 0");

  Ok(())
}

/// Example: Moving Average Transformer
///
/// This demonstrates:
/// - Sliding window state management
/// - Configurable window sizes
/// - State that maintains a queue of recent values
/// - Different window sizes produce different smoothing
pub async fn moving_average_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📊 Setting up moving average example...");

  // Create sample data: values with some variation
  let sample_data = vec![
    10.0, 12.0, 11.0, 13.0, 15.0, 14.0, 16.0, 18.0, 17.0, 19.0, 20.0, 18.0, 16.0, 15.0, 17.0,
  ];

  let _producer = streamweave::producers::vec::vec_producer::VecProducer::new(sample_data.clone())
    .with_name("data-generator".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Moving average with small window (3 items) - more responsive
  println!("\n📈 Moving Average with Window Size 3 (responsive):");
  let transformer_small = MovingAverageTransformer::new(3)
    .with_name("moving-avg-3".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let consumer_small = VecConsumer::new();

  let pipeline_small = PipelineBuilder::new()
    .producer(streamweave::producers::vec::vec_producer::VecProducer::new(
      sample_data.clone(),
    ))
    .transformer(transformer_small)
    .consumer(consumer_small);

  let (_, result_consumer_small) = pipeline_small.run().await?;
  let results_small = result_consumer_small.into_vec();

  println!("   Input values: {:?}", sample_data);
  println!("   Moving averages (window=3):");
  for (i, avg) in results_small.iter().enumerate() {
    println!("     Item {}: {:.2}", i + 1, avg);
  }

  // Moving average with large window (5 items) - smoother
  println!("\n📈 Moving Average with Window Size 5 (smoother):");
  let transformer_large = MovingAverageTransformer::new(5)
    .with_name("moving-avg-5".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let consumer_large = VecConsumer::new();

  let pipeline_large = PipelineBuilder::new()
    .producer(streamweave::producers::vec::vec_producer::VecProducer::new(
      sample_data.clone(),
    ))
    .transformer(transformer_large)
    .consumer(consumer_large);

  let (_, result_consumer_large) = pipeline_large.run().await?;
  let results_large = result_consumer_large.into_vec();

  println!("   Moving averages (window=5):");
  for (i, avg) in results_large.iter().enumerate() {
    println!("     Item {}: {:.2}", i + 1, avg);
  }

  println!("\n💡 Notice how:");
  println!("   - Window size 3 responds faster to changes");
  println!("   - Window size 5 provides smoother, less noisy output");
  println!("   - Both maintain state across all items in the stream");

  Ok(())
}

/// Example: State Checkpointing
///
/// This demonstrates:
/// - Inspecting state at different points
/// - State persistence across multiple pipeline runs
/// - State lifecycle management
pub async fn state_checkpoint_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📊 Setting up state checkpoint example...");

  // Create a running sum transformer that we'll reuse
  let transformer = RunningSumTransformer::<i32>::with_initial(100) // Start at 100
    .with_name("checkpoint-sum".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  println!("🔄 First pipeline run (starting from 100):");
  let producer1 = RangeProducer::new(1, 6, 1);
  let consumer1 = VecConsumer::new();

  let pipeline1 = PipelineBuilder::new()
    .producer(producer1)
    .transformer(transformer.clone())
    .consumer(consumer1);

  let (_, result_consumer1) = pipeline1.run().await?;
  let results1 = result_consumer1.into_vec();
  println!("   Results: {:?}", results1);
  println!("   Expected: [101, 103, 106, 110, 115] (starting from 100)");

  // Check state after first run
  if let Ok(Some(state)) = transformer.state() {
    println!("   ✅ State after first run: {}", state);
  }

  println!("\n🔄 Second pipeline run (continuing from previous state):");
  let producer2 = RangeProducer::new(6, 11, 1);
  let consumer2 = VecConsumer::new();

  let pipeline2 = PipelineBuilder::new()
    .producer(producer2)
    .transformer(transformer.clone())
    .consumer(consumer2);

  let (_, result_consumer2) = pipeline2.run().await?;
  let results2 = result_consumer2.into_vec();
  println!("   Results: {:?}", results2);
  println!("   Expected: [121, 128, 136, 145, 155] (continuing from 115)");

  // Check state after second run
  if let Ok(Some(state)) = transformer.state() {
    println!("   ✅ State after second run: {}", state);
    println!("   ✅ State persisted across pipeline runs!");
  }

  println!("\n💡 State Lifecycle:");
  println!("   1. State is initialized with initial value (100)");
  println!("   2. State persists across multiple pipeline runs");
  println!("   3. State can be inspected at any time");
  println!("   4. State can be reset to start fresh");
  println!("   5. State is thread-safe and can be shared");

  Ok(())
}
