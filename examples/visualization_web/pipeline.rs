//! Pipeline setup for web visualization example

use streamweave::{
  consumers::vec::vec_consumer::VecConsumer, producers::array::array_producer::ArrayProducer,
  transformers::map::map_transformer::MapTransformer,
};

/// Creates a sample pipeline for visualization demonstration.
///
/// This creates a more complex pipeline with multiple transformers
/// to demonstrate a more interesting DAG structure.
///
/// # Returns
///
/// A tuple containing the producer, transformer, and consumer components.
/// Note: For simplicity, we return just one transformer, but the DAG
/// will show the pipeline structure.
#[allow(clippy::type_complexity)]
pub fn create_sample_pipeline() -> (
  ArrayProducer<i32, 10>,
  MapTransformer<impl FnMut(i32) -> i32 + Send + Clone + 'static, i32, i32>,
  VecConsumer<i32>,
) {
  let producer =
    ArrayProducer::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).with_name("numbers_producer".to_string());
  let transformer = MapTransformer::new(|x: i32| x * 2).with_name("doubler".to_string());
  let consumer = VecConsumer::new().with_name("results_collector".to_string());

  (producer, transformer, consumer)
}
