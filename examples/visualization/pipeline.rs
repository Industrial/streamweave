//! Pipeline setup for visualization example

use streamweave::{
  consumers::vec::vec_consumer::VecConsumer, producers::array::array_producer::ArrayProducer,
  transformers::map::map_transformer::MapTransformer,
};

/// Creates a sample pipeline for visualization demonstration.
///
/// # Returns
///
/// A tuple containing the producer, transformer, and consumer components
/// that can be used to create a pipeline and generate a DAG.
pub fn create_sample_pipeline() -> (
  ArrayProducer<i32>,
  MapTransformer<i32, i32>,
  VecConsumer<i32>,
) {
  let producer = ArrayProducer::new(vec![1, 2, 3, 4, 5]).with_name("numbers_producer".to_string());
  let transformer = MapTransformer::new(|x| x * 2).with_name("doubler".to_string());
  let consumer = VecConsumer::new().with_name("results_collector".to_string());

  (producer, transformer, consumer)
}
