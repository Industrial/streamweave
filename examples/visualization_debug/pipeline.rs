//! Pipeline setup for visualization_debug example

use streamweave::{
  consumers::vec::vec_consumer::VecConsumer, producers::array::array_producer::ArrayProducer,
  transformers::filter::filter_transformer::FilterTransformer,
  transformers::map::map_transformer::MapTransformer,
};

/// Creates a multi-stage pipeline for debug visualization demonstration.
#[allow(clippy::type_complexity)]
pub fn create_multi_stage_pipeline() -> (
  ArrayProducer<i32, 10>,
  MapTransformer<impl FnMut(i32) -> i32 + Send + Clone + 'static, i32, i32>,
  FilterTransformer<impl FnMut(&i32) -> bool + Send + Clone + 'static, i32>,
  VecConsumer<i32>,
) {
  let producer =
    ArrayProducer::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).with_name("numbers_producer".to_string());

  let transformer1 = MapTransformer::new(|x: i32| x * 2).with_name("doubler".to_string());

  let transformer2 =
    FilterTransformer::new(|x: &i32| x % 2 == 0).with_name("even_filter".to_string());

  let consumer = VecConsumer::new().with_name("results_collector".to_string());

  (producer, transformer1, transformer2, consumer)
}
