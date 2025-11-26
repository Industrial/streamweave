use streamweave::{
  consumers::vec::vec_consumer::VecConsumer,
  pipeline::PipelineBuilder,
  producers::range::range_producer::RangeProducer,
  transformers::map::map_transformer::MapTransformer,
  transformer::{Transformer, TransformerConfig},
};

#[tokio::test]
async fn test_basic_pipeline_doubles_numbers() {
  // Create a pipeline that doubles numbers from 1 to 5
  let mut map_transformer = MapTransformer::new(|x: i32| x * 2);
  map_transformer.set_config(TransformerConfig::default().with_name("doubler".to_string()));
  
  let pipeline = PipelineBuilder::new()
    .producer(RangeProducer::new(1, 6, 1))
    .transformer(map_transformer)
    .consumer(VecConsumer::<i32>::new());

  // Run the pipeline
  let result = pipeline.run().await.unwrap();
  let numbers = result.1.into_vec();
  
  // Verify the results
  assert_eq!(numbers, vec![2, 4, 6, 8, 10]);
}

#[tokio::test]
async fn test_basic_pipeline_with_different_range() {
  // Test with a different range
  let mut map_transformer = MapTransformer::new(|x: i32| x * 3);
  map_transformer.set_config(TransformerConfig::default().with_name("tripler".to_string()));
  
  let pipeline = PipelineBuilder::new()
    .producer(RangeProducer::new(0, 4, 1))
    .transformer(map_transformer)
    .consumer(VecConsumer::<i32>::new());

  let result = pipeline.run().await.unwrap();
  let numbers = result.1.into_vec();
  
  // Verify the results
  assert_eq!(numbers, vec![0, 3, 6, 9]);
}
