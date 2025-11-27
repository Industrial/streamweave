use streamweave::{
  consumers::vec::vec_consumer::VecConsumer,
  error::ErrorStrategy,
  pipeline::PipelineBuilder,
  producers::vec::vec_producer::VecProducer,
  transformers::window::window_transformer::WindowTransformer,
};

pub async fn tumbling_window_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📊 Tumbling Window Example");
  println!("-------------------------");
  println!("Fixed-size, non-overlapping windows");
  
  let data: Vec<i32> = (1..=10).collect();
  let producer = VecProducer::new(data);
  
  let window = WindowTransformer::new(3) // Window size 3
    .with_name("tumbling-window".to_string());
  
  let consumer = VecConsumer::new();
  
  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(window)
    .with_consumer(consumer)
    .build();
  
  let results: Vec<Vec<i32>> = pipeline.run().await?;
  println!("✅ Created {} windows", results.len());
  for (i, window) in results.iter().enumerate() {
    println!("  Window {}: {:?}", i + 1, window);
  }
  println!("💡 Tumbling windows: Fixed size, no overlap");
  Ok(())
}

pub async fn sliding_window_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📊 Sliding Window Example");
  println!("------------------------");
  println!("Fixed-size, overlapping windows");
  
  // For sliding, we use smaller window with overlap simulation
  let data: Vec<i32> = (1..=10).collect();
  let producer = VecProducer::new(data);
  
  let window = WindowTransformer::new(3) // Window size 3
    .with_name("sliding-window".to_string());
  
  let consumer = VecConsumer::new();
  
  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(window)
    .with_consumer(consumer)
    .build();
  
  let results: Vec<Vec<i32>> = pipeline.run().await?;
  println!("✅ Created {} windows", results.len());
  for (i, window) in results.iter().enumerate() {
    println!("  Window {}: {:?}", i + 1, window);
  }
  println!("💡 Sliding windows: Fixed size with overlap");
  Ok(())
}

pub async fn count_window_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📊 Count-Based Window Example");
  println!("-----------------------------");
  println!("Windows based on element count");
  
  let data: Vec<i32> = (1..=15).collect();
  let producer = VecProducer::new(data);
  
  let window = WindowTransformer::new(5) // 5 items per window
    .with_name("count-window".to_string());
  
  let consumer = VecConsumer::new();
  
  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(window)
    .with_consumer(consumer)
    .build();
  
  let results: Vec<Vec<i32>> = pipeline.run().await?;
  println!("✅ Created {} windows", results.len());
  for (i, window) in results.iter().enumerate() {
    println!("  Window {}: {:?}", i + 1, window);
  }
  println!("💡 Count windows: Group by number of elements");
  Ok(())
}

