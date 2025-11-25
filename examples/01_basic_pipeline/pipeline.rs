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
    .consumer(ConsoleConsumer::new());

  // Run the pipeline
  pipeline.run().await?;

  Ok(())
}
