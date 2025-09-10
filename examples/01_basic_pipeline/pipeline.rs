use streamweave::{
  consumers::console::console_consumer::ConsoleConsumer, pipeline::PipelineBuilder,
  producers::range::range_producer::RangeProducer,
  transformers::map::map_transformer::MapTransformer,
};

/// Creates and runs a basic pipeline that doubles numbers from 1 to 5
pub async fn run_basic_pipeline() -> Result<(), Box<dyn std::error::Error>> {
  // Create a pipeline that:
  // 1. Produces numbers from 1 to 5
  // 2. Doubles each number
  // 3. Prints the result to the console
  let pipeline = PipelineBuilder::new()
    .producer(RangeProducer::new(1, 6, 1))
    .transformer(MapTransformer::new(|x: i32| x * 2))
    ._consumer(ConsoleConsumer::new());

  // Run the pipeline
  pipeline.run().await?;

  Ok(())
}

/// Creates a pipeline that doubles numbers from a given range
pub fn create_doubling_pipeline(start: i32, end: i32, step: i32) {
  PipelineBuilder::new()
    .producer(RangeProducer::new(start, end, step))
    .transformer(MapTransformer::new(|x: i32| x * 2))
    ._consumer(ConsoleConsumer::new());
}

/// Creates a pipeline that multiplies numbers by a given factor
pub fn create_multiplying_pipeline(start: i32, end: i32, step: i32, factor: i32) {
  PipelineBuilder::new()
    .producer(RangeProducer::new(start, end, step))
    .transformer(MapTransformer::new(move |x: i32| x * factor))
    ._consumer(ConsoleConsumer::new());
}
