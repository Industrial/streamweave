use streamweave::{
  consumers::vec::vec_consumer::VecConsumer,
  error::{ErrorAction, ErrorStrategy, StreamError},
  pipeline::PipelineBuilder,
  producers::vec::vec_producer::VecProducer,
  transformers::map::map_transformer::MapTransformer,
};

/// Example: Stop Strategy
///
/// This demonstrates:
/// - Pipeline stops immediately on first error
/// - No partial results are returned
/// - Useful for critical data where errors are unacceptable
pub async fn stop_strategy_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("🛑 Setting up Stop strategy example...");

  // Create data with some items that will cause errors
  let data = vec![1, 2, -1, 4, 5]; // -1 will cause an error in our transformer

  let producer = VecProducer::new(data)
    .with_name("number-producer".to_string())
    .with_error_strategy(ErrorStrategy::Stop);

  // Transformer that fails on negative numbers
  let transformer = MapTransformer::new(|x: i32| -> Result<i32, String> {
    if x < 0 {
      Err(format!("Negative number not allowed: {}", x))
    } else {
      Ok(x * 2)
    }
  })
  .with_error_strategy(ErrorStrategy::Stop); // Stop on first error

  let consumer = VecConsumer::new();

  println!("🔄 Processing with Stop strategy...");
  println!("   Input: [1, 2, -1, 4, 5]");
  println!("   Expected: Pipeline stops at -1, only [2, 4] processed");
  println!();

  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(transformer)
    .with_consumer(consumer)
    .build();

  let results = pipeline.run().await?;

  println!("✅ Stop strategy completed!");
  println!("📊 Results ({} items):", results.len());
  for (i, result) in results.iter().enumerate() {
    println!("  {}. {}", i + 1, result);
  }

  println!("\n💡 Stop Strategy:");
  println!("   - Pipeline stops immediately on first error");
  println!("   - No partial results after error");
  println!("   - Use when errors indicate critical failure");
  println!("   - Best for: Data integrity requirements, critical pipelines");

  Ok(())
}

/// Example: Skip Strategy
///
/// This demonstrates:
/// - Errors are skipped, processing continues
/// - Partial results are returned
/// - Useful for non-critical errors or data cleaning
pub async fn skip_strategy_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("⏭️  Setting up Skip strategy example...");

  // Create data with some items that will cause errors
  let data = vec![1, 2, -1, 4, -2, 5, 6]; // Negative numbers will cause errors

  let producer = VecProducer::new(data)
    .with_name("number-producer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Transformer that fails on negative numbers
  let transformer = MapTransformer::new(|x: i32| -> Result<i32, String> {
    if x < 0 {
      Err(format!("Negative number not allowed: {}", x))
    } else {
      Ok(x * 2)
    }
  })
  .with_error_strategy(ErrorStrategy::Skip); // Skip errors and continue

  let consumer = VecConsumer::new();

  println!("🔄 Processing with Skip strategy...");
  println!("   Input: [1, 2, -1, 4, -2, 5, 6]");
  println!("   Expected: Skips -1 and -2, processes [1, 2, 4, 5, 6]");
  println!();

  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(transformer)
    .with_consumer(consumer)
    .build();

  let results = pipeline.run().await?;

  println!("✅ Skip strategy completed!");
  println!("📊 Results ({} items, errors skipped):", results.len());
  for (i, result) in results.iter().enumerate() {
    println!("  {}. {}", i + 1, result);
  }

  println!("\n💡 Skip Strategy:");
  println!("   - Errors are skipped, processing continues");
  println!("   - Partial results are returned");
  println!("   - Use when errors are non-critical");
  println!("   - Best for: Data cleaning, filtering invalid records, logging");

  Ok(())
}

/// Example: Retry Strategy
///
/// This demonstrates:
/// - ErrorStrategy::Retry for automatic retries
/// - Errors trigger retries up to a maximum count
/// - Useful for transient failures
/// - After max retries, falls back to Stop
pub async fn retry_strategy_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("🔄 Setting up Retry strategy example...");

  let data = vec![1, 2, 3, 4, 5];

  let producer = VecProducer::new(data)
    .with_name("number-producer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Transformer that may fail (simulating transient errors)
  // In a real scenario, this would be an external call that might fail
  let transformer = MapTransformer::new(|x: i32| -> Result<i32, String> {
    // Simulate transient failure for even numbers
    if x % 2 == 0 {
      Err(format!("Transient error for {}, will retry", x))
    } else {
      Ok(x * 2)
    }
  })
  .with_error_strategy(ErrorStrategy::Retry(3)); // Retry up to 3 times

  let consumer = VecConsumer::new();

  println!("🔄 Processing with Retry strategy...");
  println!("   Input: [1, 2, 3, 4, 5]");
  println!("   Simulated: Even numbers fail (transient errors)");
  println!("   Retry limit: 3 (after which it stops)");
  println!();

  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(transformer)
    .with_consumer(consumer)
    .build();

  let results = pipeline.run().await?;

  println!("✅ Retry strategy completed!");
  println!("📊 Results ({} items after retries):", results.len());
  for (i, result) in results.iter().enumerate() {
    println!("  {}. {}", i + 1, result);
  }

  println!("\n💡 Retry Strategy:");
  println!("   - Errors trigger retries up to maximum count");
  println!("   - Useful for transient failures (network, timeouts)");
  println!("   - After max retries, falls back to Stop");
  println!("   - Best for: Network calls, external services, temporary failures");
  println!("   - Note: Actual retry behavior depends on component implementation");

  Ok(())
}

/// Example: Custom Strategy
///
/// This demonstrates:
/// - Custom error handling logic
/// - Conditional error handling based on error type or context
/// - Complex error recovery patterns
pub async fn custom_strategy_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("🎯 Setting up Custom strategy example...");

  let data = vec![1, 2, -1, 4, -2, 5, 100, 6]; // Mix of negative and large numbers

  let producer = VecProducer::new(data)
    .with_name("number-producer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Custom strategy: Skip negative numbers, but stop on numbers > 50
  let custom_strategy = ErrorStrategy::new_custom(|error: &StreamError<i32>| {
    let error_msg = error.source.to_string();
    
    if error_msg.contains("Negative number") {
      // Skip negative numbers
      ErrorAction::Skip
    } else if error_msg.contains("Too large") {
      // Stop on numbers that are too large
      ErrorAction::Stop
    } else {
      // Default to skip for other errors
      ErrorAction::Skip
    }
  });

  // Transformer with different error conditions
  let transformer = MapTransformer::new(|x: i32| -> Result<i32, String> {
    if x < 0 {
      Err(format!("Negative number not allowed: {}", x))
    } else if x > 50 {
      Err(format!("Too large: {}", x))
    } else {
      Ok(x * 2)
    }
  })
  .with_error_strategy(custom_strategy);

  let consumer = VecConsumer::new();

  println!("🔄 Processing with Custom strategy...");
  println!("   Input: [1, 2, -1, 4, -2, 5, 100, 6]");
  println!("   Strategy: Skip negatives, Stop on numbers > 50");
  println!("   Expected: Processes [1, 2, 4, 5], stops at 100");
  println!();

  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(transformer)
    .with_consumer(consumer)
    .build();

  let results = pipeline.run().await?;

  println!("✅ Custom strategy completed!");
  println!("📊 Results ({} items):", results.len());
  for (i, result) in results.iter().enumerate() {
    println!("  {}. {}", i + 1, result);
  }

  println!("\n💡 Custom Strategy:");
  println!("   - Implement custom error handling logic");
  println!("   - Make decisions based on error type or context");
  println!("   - Can combine multiple strategies conditionally");
  println!("   - Best for: Complex error scenarios, conditional recovery");

  Ok(())
}

/// Example: Component-Level Error Handling
///
/// This demonstrates:
/// - Different error strategies at component level
/// - Overriding pipeline-level strategy per component
/// - Fine-grained error control
pub async fn component_level_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("🔧 Setting up Component-level error handling example...");

  let data = vec![1, 2, -1, 4, 5];

  // Producer with Skip strategy
  let producer = VecProducer::new(data)
    .with_name("producer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // First transformer: Skip errors (component-level override)
  let transformer1 = MapTransformer::new(|x: i32| -> Result<i32, String> {
    if x < 0 {
      Err(format!("Negative in transformer1: {}", x))
    } else {
      Ok(x * 2)
    }
  })
  .with_error_strategy(ErrorStrategy::Skip); // Component-level: Skip

  // Second transformer: Stop on errors (different strategy)
  let transformer2 = MapTransformer::new(|x: i32| -> Result<i32, String> {
    if x > 10 {
      Err(format!("Too large in transformer2: {}", x))
    } else {
      Ok(x + 10)
    }
  })
  .with_error_strategy(ErrorStrategy::Stop); // Component-level: Stop

  let consumer = VecConsumer::new();

  println!("🔄 Processing with component-level strategies...");
  println!("   Input: [1, 2, -1, 4, 5]");
  println!("   Transformer1: Skip errors (will skip -1)");
  println!("   Transformer2: Stop errors (will stop if x > 10)");
  println!();

  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(transformer1)
    .with_transformer(transformer2)
    .with_consumer(consumer)
    .build();

  let results = pipeline.run().await?;

  println!("✅ Component-level error handling completed!");
  println!("📊 Results ({} items):", results.len());
  for (i, result) in results.iter().enumerate() {
    println!("  {}. {}", i + 1, result);
  }

  println!("\n💡 Component-Level Error Handling:");
  println!("   - Each component can have its own error strategy");
  println!("   - Overrides pipeline-level strategy");
  println!("   - Allows fine-grained error control");
  println!("   - Best for: Different error tolerance per stage");

  Ok(())
}

