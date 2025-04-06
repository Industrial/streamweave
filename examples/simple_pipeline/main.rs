use streamweave::{
  consumers::console::ConsoleConsumer, pipeline::PipelineBuilder, producers::range::RangeProducer,
  traits::consumer::Consumer, transformers::map::MapTransformer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let range_producer = RangeProducer::new(1, 6, 1);
  let map_transformer = MapTransformer::new(|x: i32| Ok(x * x));
  let console_consumer = ConsoleConsumer::default();

  let pipeline = PipelineBuilder::new().producer(range_producer);

  let pipeline1 = pipeline.transformer(map_transformer);

  // .transformer(map_transformer)
  // .consumer(console_consumer);

  match pipeline.run().await {
    Ok((_, consumer)) => {
      println!("Pipeline completed successfully");
      println!("Consumer info: {:?}", consumer.component_info());
    }
    Err(e) => {
      eprintln!("Pipeline failed: {}", e);
      eprintln!("Error context: {:?}", e.context());
      eprintln!("Component info: {:?}", e.component());
    }
  }

  Ok(())
}
