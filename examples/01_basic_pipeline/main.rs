use streamweave::{
  consumers::console::ConsoleConsumer, pipeline::PipelineBuilder, producers::range::RangeProducer,
  transformers::map::MapTransformer,
};

#[tokio::main]
async fn main() {
  // Create a pipeline that:
  // 1. Produces numbers from 1 to 5
  // 2. Doubles each number
  // 3. Prints the result to the console
  let pipeline = PipelineBuilder::new()
    .producer(RangeProducer::new(1, 6, 1))
    .transformer(MapTransformer::new(|x: i32| x * 2))
    .consumer(ConsoleConsumer::new());

  // Run the pipeline
  pipeline.run().await.unwrap();
}
